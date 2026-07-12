import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { AppShell } from './app/AppShell';
import { BootstrapErrorBoundary } from './features/diagnostics/BootstrapErrorBoundary';
import './styles/base.css';
import './styles/accessibility.css';

const root = document.getElementById('root');
if (!root) throw new Error('缺少应用根节点 #root');
createRoot(root).render(<StrictMode><BootstrapErrorBoundary><AppShell /></BootstrapErrorBoundary></StrictMode>);
