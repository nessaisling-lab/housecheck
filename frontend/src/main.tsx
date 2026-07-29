import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router'
import './index.css'
import App from './App.tsx'
import { applyTextSize } from '@/lib/store'

// Apply the saved reader text size before the first paint, so the app never
// flashes at the default size and then jump-resizes (WCAG 1.4.4).
applyTextSize()

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </StrictMode>,
)
