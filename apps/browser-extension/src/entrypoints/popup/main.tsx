import React from 'react';
import ReactDOM from 'react-dom/client';
import { ThemeProvider } from '@/ui/theme/ThemeProvider';
import { I18nProvider } from '@/ui/i18n/I18nProvider';
import { ErrorBoundary } from '@/ui/components/ErrorBoundary';
import PopupApp from './PopupApp';
import '../../style.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <ThemeProvider>
        <I18nProvider>
          <PopupApp />
        </I18nProvider>
      </ThemeProvider>
    </ErrorBoundary>
  </React.StrictMode>
);
