import { createBrowserRouter } from 'react-router';
import { App } from './App';
import { Login } from './auth/Login';
import { RequireAuth } from './auth/RequireAuth';
import { Dashboard } from './pages/Dashboard';
import { Servers } from './pages/Servers';
import { Projects } from './pages/Projects';
import { Applications } from './pages/Applications';
import { ApplicationDetail } from './pages/ApplicationDetail';
import { Deployments } from './pages/Deployments';
import { Settings } from './pages/Settings';

export const router = createBrowserRouter([
  {
    path: '/',
    element: <App />,
    children: [
      { path: 'login', element: <Login /> },
      {
        element: <RequireAuth />,
        children: [
          { index: true, element: <Dashboard /> },
          { path: 'servers', element: <Servers /> },
          { path: 'projects', element: <Projects /> },
          { path: 'applications', element: <Applications /> },
          { path: 'applications/:id', element: <ApplicationDetail /> },
          { path: 'deployments', element: <Deployments /> },
          { path: 'settings', element: <Settings /> },
        ],
      },
    ],
  },
]);
