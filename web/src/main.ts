import { createApp } from 'vue'
import { createRouter, createWebHistory } from 'vue-router'
import App from './App.vue'
import './style.css'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'tray', component: () => import('./views/TrayView.vue') },
    { path: '/prints', name: 'prints', component: () => import('./views/PrintsView.vue') },
    { path: '/prints/:id', name: 'print', component: () => import('./views/PrintView.vue'), props: true },
  ],
})

createApp(App).use(router).mount('#app')
