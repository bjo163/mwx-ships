import React from 'react';
import { NavLink, Outlet, useNavigate } from 'react-router';
import { clearToken, getToken } from './auth/token';
import {
  Server,
  Layers,
  Box,
  Rocket,
  Settings,
  Activity,
  LogOut,
  Terminal,
  Shield,
  Moon,
} from 'lucide-react';

export function App() {
  const navigate = useNavigate();
  const isAuthenticated = getToken() !== null;

  function handleLogout() {
    clearToken();
    navigate('/login');
  }

  const navItems = [
    { to: '/', label: 'Dashboard', icon: Activity, end: true },
    { to: '/servers', label: 'Servers', icon: Server },
    { to: '/projects', label: 'Projects', icon: Layers },
    { to: '/applications', label: 'Applications', icon: Box },
    { to: '/deployments', label: 'Deployments', icon: Rocket },
    { to: '/settings', label: 'Settings', icon: Settings },
  ];

  return (
    <div style={{ display: 'flex', minHeight: '100vh', background: 'var(--bg-primary)' }}>
      {/* Sidebar */}
      <aside
        style={{
          width: '260px',
          background: 'var(--bg-secondary)',
          borderRight: '1px solid var(--border-color)',
          display: 'flex',
          flexDirection: 'column',
          padding: '24px 16px',
        }}
      >
        {/* Brand */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '32px', paddingLeft: '8px' }}>
          <div
            style={{
              width: '40px',
              height: '40px',
              borderRadius: '10px',
              background: 'linear-gradient(135deg, #6366f1, #06b6d4)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              boxShadow: '0 0 16px rgba(99, 102, 241, 0.4)',
            }}
          >
            <Moon size={22} color="#fff" />
          </div>
          <div>
            <h1 style={{ fontSize: '1.25rem', fontWeight: 700, letterSpacing: '-0.02em', color: '#fff' }}>
              MOONSHIPS
            </h1>
            <p style={{ fontSize: '0.7rem', color: 'var(--cyan-primary)', fontWeight: 600, letterSpacing: '0.05em' }}>
              SQLITE-FIRST MINI-PAAS
            </p>
          </div>
        </div>

        {/* Navigation Links */}
        <nav style={{ display: 'flex', flexDirection: 'column', gap: '4px', flex: 1 }}>
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.end}
                style={({ isActive }) => ({
                  display: 'flex',
                  alignItems: 'center',
                  gap: '12px',
                  padding: '10px 14px',
                  borderRadius: '8px',
                  color: isActive ? '#fff' : 'var(--text-secondary)',
                  background: isActive ? 'var(--accent-glow)' : 'transparent',
                  border: isActive ? '1px solid rgba(99, 102, 241, 0.3)' : '1px solid transparent',
                  textDecoration: 'none',
                  fontSize: '0.9rem',
                  fontWeight: isActive ? 600 : 500,
                  transition: 'all 0.15s ease',
                })}
              >
                <Icon size={18} />
                <span>{item.label}</span>
              </NavLink>
            );
          })}
        </nav>

        {/* Control Plane Footer */}
        <div style={{ paddingTop: '16px', borderTop: '1px solid var(--border-color)', display: 'flex', flexDirection: 'column', gap: '12px' }}>
          <div
            style={{
              padding: '10px 12px',
              borderRadius: '8px',
              background: 'rgba(255, 255, 255, 0.03)',
              border: '1px solid var(--border-color)',
              fontSize: '0.75rem',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', color: 'var(--success)', marginBottom: '4px', fontWeight: 600 }}>
              <span className="pulse-dot" />
              <span>Control Plane Active</span>
            </div>
            <div style={{ color: 'var(--text-muted)' }}>SQLite Engine · Persistent Queue</div>
          </div>

          {isAuthenticated && (
            <button
              onClick={handleLogout}
              className="btn btn-secondary"
              style={{ width: '100%', justifyContent: 'flex-start' }}
            >
              <LogOut size={16} />
              <span>Sign Out</span>
            </button>
          )}
        </div>
      </aside>

      {/* Main Content Area */}
      <main style={{ flex: 1, padding: '32px 40px', overflowY: 'auto' }}>
        <Outlet />
      </main>
    </div>
  );
}
