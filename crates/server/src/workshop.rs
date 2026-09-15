//! The one thread that owns the tray.
//!
//! Everything that changes the bath arrives here as a [`Command`] over a
//! channel and is handled strictly one at a time. That single ordering is the
//! whole consistency story: every op gets the next sequence number, and every
//! browser applies ops in sequence order, so no two trays can disagree.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use marbling::{Op, Tray};
use serde::Serialize;
use tokio::sync::{broadcast, oneshot};

use crate::patterns::{Composition, Tally, next_ground};
use crate::print;
use crate::random::Random;
use crate::store::{Draft, Print, Recipe, Store};

pub type PersonId = u32;

/// Who did something: a person, or the tray marbling by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(untagged)]
pub enum Hand {
    Person(PersonId),
    #[serde(serialize_with = "tray_hand")]
    Tray,
}

fn tray_hand<S: serde::Serializer>(s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str("tray")
}

/// What the server tells every browser, already encoded once as JSON.
pub type Announcement = Arc<str>;

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event<'a> {
    Op { seq: u64, op: &'a Op, by: Hand },
    /// Someone laid a sheet; the tray is about to be skimmed.
    Laying { by: Hand, name: &'a str },
    /// The print is dry and in the gallery.
    Printed { print: &'a Print },
    /// After a quiet spell: what a browser's tray should hash to at `seq`.
    Checksum { seq: u64, sum: String },
    /// Only the person it's addressed to shows it.
    Refused { to: Hand, reason: &'a str },
}

pub fn announce(event: &Event) -> Announcement {
    serde_json::to_string(event).expect("events always serialize").into()
}

#[derive(Debug)]
pub enum Refusal {
    Invalid(String),
    TooFast,
    Full,
    TooSoon,
    Busy,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Invalid(why) => write!(f, "The tray refused that: {why}."),
            Refusal::TooFast => write!(f, "Slow down a little; the ink needs a moment to settle."),
            Refusal::Full => write!(f, "The tray is full. Lay a sheet to start a new pattern."),
            Refusal::TooSoon => write!(f, "There's hardly anything on the tray yet. Add some ink first."),
            Refusal::Busy => write!(f, "A sheet is already being laid."),
        }
    }
}

pub enum Command {
    Op { by: PersonId, op: Op },
    Snapshot { reply: oneshot::Sender<Vec<u8>> },
    Lay { by: PersonId },
    Printed(Result<Print, String>),
    Stop,
}

pub struct Workshop {
    commands: mpsc::Sender<Command>,
    events: broadcast::Sender<Announcement>,
    people: Arc<AtomicUsize>,
}

impl Workshop {
    pub fn start(store: Store, idle_after: Option<Duration>) -> Arc<Workshop> {
        let (commands, inbox) = mpsc::channel();
        let (events, _) = broadcast::channel(8192);
        let people = Arc::new(AtomicUsize::new(0));
        let workshop = Arc::new(Workshop { commands: commands.clone(), events: events.clone(), people: people.clone() });
        std::thread::Builder::new()
            .name("workshop".into())
            .spawn(move || Bench::new(store, events, commands, people, idle_after).run(inbox))
            .expect("spawn the workshop thread");
        workshop
    }

    pub fn send(&self, command: Command) {
        // The workshop only goes away when the process does.
        let _ = self.commands.send(command);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Announcement> {
        self.events.subscribe()
    }

    /// Tells everyone something that doesn't touch the tray, like where a
    /// cursor is.
    pub fn broadcast(&self, announcement: Announcement) {
        let _ = self.events.send(announcement);
    }

    pub async fn snapshot(&self) -> Vec<u8> {
        let (reply, answer) = oneshot::channel();
        self.send(Command::Snapshot { reply });
        answer.await.unwrap_or_default()
    }

    pub fn arrived(&self) {
        self.people.fetch_add(1, Ordering::Relaxed);
    }

    pub fn left(&self) {
        self.people.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn people(&self) -> usize {
        self.people.load(Ordering::Relaxed)
    }
}

/// How many ops a person may make in a burst, and how fast that refills.
/// Browsers send a drag as at most ~25 strokes a second, so this lets anyone
/// work vigorously while stopping a script from flooding the bath.
const BURST: f64 = 200.0;
const REFILL_PER_SECOND: f64 = 60.0;

struct Allowance {
    tokens: f64,
    at: Instant,
}

/// The workshop thread's state.
struct Bench {
    tray: Tray,
    recipe: Recipe,
    tally: Tally,
    hands: HashSet<PersonId>,
    allowances: HashMap<PersonId, Allowance>,
    store: Store,
    events: broadcast::Sender<Announcement>,
    commands: mpsc::Sender<Command>,
    people: Arc<AtomicUsize>,
    random: Random,
    idle_after: Option<Duration>,
    last_human: Instant,
    last_op: Instant,
    checksum_sent_for: u64,
    composition: Option<Composition>,
    resting_since: Option<Instant>,
    /// Compositions the tray has finished since the bath was last skimmed.
    movements: u32,
    last_print: Instant,
    laying: bool,
}

impl Bench {
    fn new(
        store: Store,
        events: broadcast::Sender<Announcement>,
        commands: mpsc::Sender<Command>,
        people: Arc<AtomicUsize>,
        idle_after: Option<Duration>,
    ) -> Self {
        let mut random = Random::from_time();
        let ground = next_ground(255, &mut random);
        Bench {
            tray: Tray::new(ground),
            recipe: Recipe { ground, ops: Vec::new() },
            tally: Tally::default(),
            hands: HashSet::new(),
            allowances: HashMap::new(),
            store,
            events,
            commands,
            people,
            random,
            idle_after,
            last_human: Instant::now(),
            last_op: Instant::now(),
            checksum_sent_for: u64::MAX,
            composition: None,
            resting_since: None,
            movements: 0,
            last_print: Instant::now().checked_sub(Duration::from_secs(3600)).unwrap_or_else(Instant::now),
            laying: false,
        }
    }

    fn run(mut self, inbox: mpsc::Receiver<Command>) {
        loop {
            match inbox.recv_timeout(Duration::from_millis(110)) {
                Ok(Command::Op { by, op }) => self.human_op(by, op),
                Ok(Command::Snapshot { reply }) => {
                    let _ = reply.send(marbling::encode(&self.tray));
                }
                Ok(Command::Lay { by }) => self.lay(Hand::Person(by)),
                Ok(Command::Printed(result)) => self.printed(result),
                Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => self.quiet(),
            }
        }
    }

    fn tell(&self, event: &Event) {
        // No receivers just means nobody is watching.
        let _ = self.events.send(announce(event));
    }

    fn refuse(&self, to: Hand, reason: &Refusal) {
        self.tell(&Event::Refused { to, reason: &reason.to_string() });
    }

    fn human_op(&mut self, by: PersonId, op: Op) {
        if let Err(why) = self.check(by, &op) {
            self.refuse(Hand::Person(by), &why);
            return;
        }
        self.last_human = Instant::now();
        self.composition = None;
        self.resting_since = None;
        self.hands.insert(by);
        self.apply(op, Hand::Person(by));
    }

    fn check(&mut self, by: PersonId, op: &Op) -> Result<(), Refusal> {
        op.validate().map_err(|e| Refusal::Invalid(e.to_string()))?;
        if matches!(op, Op::Skim { .. }) {
            return Err(Refusal::Invalid("only laying a sheet skims the bath".into()));
        }
        if self.laying {
            return Err(Refusal::Busy);
        }
        if self.tray.is_full() {
            return Err(Refusal::Full);
        }
        let now = Instant::now();
        let allowance = self.allowances.entry(by).or_insert(Allowance { tokens: BURST, at: now });
        allowance.tokens = (allowance.tokens + now.duration_since(allowance.at).as_secs_f64() * REFILL_PER_SECOND).min(BURST);
        allowance.at = now;
        if allowance.tokens < 1.0 {
            return Err(Refusal::TooFast);
        }
        allowance.tokens -= 1.0;
        Ok(())
    }

    fn apply(&mut self, op: Op, by: Hand) {
        self.tally.count(&op, &self.tray);
        self.tray.apply(&op);
        self.last_op = Instant::now();
        match op {
            Op::Skim { ground } => {
                self.recipe = Recipe { ground, ops: Vec::new() };
                self.movements = 0;
            }
            _ => self.recipe.ops.push(op.clone()),
        }
        self.tell(&Event::Op { seq: self.tray.seq, op: &op, by });
    }

    fn lay(&mut self, by: Hand) {
        if self.laying {
            self.refuse(by, &Refusal::Busy);
            return;
        }
        if self.tally.ops() < 8 {
            self.refuse(by, &Refusal::TooSoon);
            return;
        }
        self.laying = true;
        let pattern = self.tally.pattern();
        let people = self.hands.len() as u32;
        let tally = self.tally.clone();
        let tray = self.tray.clone();
        let recipe = Recipe { ground: self.recipe.ground, ops: std::mem::take(&mut self.recipe.ops) };
        self.tell(&Event::Laying { by, name: pattern });

        let store = self.store.clone();
        let done = self.commands.clone();
        std::thread::spawn(move || {
            let image = print::png(&tray, 2400);
            let small = print::png(&tray, 640);
            let made_at = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
            let result = store
                .save(Draft { pattern, people, tally, recipe, image, small, made_at })
                .map_err(|e| e.to_string());
            let _ = done.send(Command::Printed(result));
        });

        // The bath is skimmed at once so marbling can carry on while the print dries.
        self.hands.clear();
        let ground = next_ground(self.tray.ground, &mut self.random);
        self.apply(Op::Skim { ground }, by);
        self.last_print = Instant::now();
    }

    fn printed(&mut self, result: Result<Print, String>) {
        self.laying = false;
        match result {
            Ok(print) => {
                tracing::info!(id = print.id, name = %print.name, "print hung in the gallery");
                self.tell(&Event::Printed { print: &print });
            }
            Err(error) => {
                tracing::error!(%error, "couldn't save the print");
            }
        }
    }

    /// Nothing arrived for a moment: confirm checksums, and maybe marble alone.
    fn quiet(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_op) > Duration::from_millis(1200) && self.checksum_sent_for != self.tray.seq {
            self.checksum_sent_for = self.tray.seq;
            self.tell(&Event::Checksum { seq: self.tray.seq, sum: format!("{:016x}", self.tray.checksum()) });
        }

        let Some(idle_after) = self.idle_after else { return };
        if self.laying || self.people.load(Ordering::Relaxed) == 0 || now.duration_since(self.last_human) < idle_after {
            return;
        }

        // Mid-composition: one op per quiet tick, at an unhurried pace.
        if self.composition.is_some() {
            if let Some(op) = self.composition.as_mut().and_then(|c| c.ops.pop_front()) {
                self.apply(op, Hand::Tray);
            }
            if self.composition.as_ref().is_some_and(|c| c.ops.is_empty()) {
                self.composition = None;
                self.resting_since = Some(now);
                self.movements += 1;
            }
            return;
        }

        // Between compositions, pause to let people look.
        if self.resting_since.is_some_and(|t| now.duration_since(t) < Duration::from_secs(10)) {
            return;
        }

        // A pattern the tray made with nobody's help is printed once it has
        // drops and at least one movement through them, but not so often that
        // the gallery fills with the tray's own work.
        if self.movements >= 2 && self.hands.is_empty() && now.duration_since(self.last_print) > Duration::from_secs(600) {
            self.lay(Hand::Tray);
            self.resting_since = Some(now);
            return;
        }

        if self.tray.is_full() {
            return;
        }
        let plan = Composition::plan(&self.tray, &mut self.random);
        tracing::info!(pattern = plan.pattern, ops = plan.ops.len(), "the tray starts marbling by itself");
        self.composition = Some(plan);
    }
}
