import { RouterProvider } from 'react-router';
import { AppProvider } from './provider';
import { router } from './router';
import { UnauthorizedRedirect } from './unauthorized-redirect';

export function App() {
  return (
    <AppProvider>
      <UnauthorizedRedirect router={router} />
      <RouterProvider router={router} />
    </AppProvider>
  );
}
