import React, { useEffect, useState } from 'react';
import { Link } from 'react-router';
import { apiRequest } from '../api/client';
import {
  Server,
  Box,
  Rocket,
  Activity,
  ArrowRight,
  Database,
  CheckCircle2,
  AlertTriangle,
  Clock,
  Layers,
} from 'lucide-react';

export function Dashboard() {
  const [stats, setStats] = useState({
    servers: 0,
    projects: 0,
    applications: 0,
  });
  const [applications, setApplications] = useState<any[]>([]);
  const [health, setHealth] = useState<any>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadData() {
      try {
        const [servers, projects, apps, h] = await Promise.all([
          apiRequest<any[]>('/api/servers').catch(() => []),
          apiRequest<any[]>('/api/projects').catch(() => []),
          apiRequest<any[]>('/api/applications').catch(() => []),
          apiRequest<any>('/api/health').catch(() => null),
        ]);

        setStats({
          servers: servers.length,
          projects: projects.length,
          applications: apps.length,
        });
        setApplications(apps);
        setHealth(h);
      } finally {
        setLoading(false);
      }
    }

    loadData();
    const interval = setInterval(loadData, 5000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div style={{ maxWidth: '1200px', margin: '0 auto' }}>
      {/* Top Welcome Banner */}
      <div style={{ marginBottom: '32px' }}>
        <h1 style={{ fontSize: '2rem', fontWeight: 700, letterSpacing: '-0.02em', marginBottom: '8px' }}>
          Infrastructure Overview
        </h1>
        <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem' }}>
          Deploy. Control. Own your infrastructure with zero external database dependencies.
        </p>
      </div>

      {/* Metrics Row */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))',
          gap: '20px',
          marginBottom: '32px',
        }}
      >
        {/* Servers Card */}
        <div className="glass-panel" style={{ padding: '20px 24px' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
            <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', fontWeight: 500 }}>Total Servers</span>
            <div style={{ padding: '8px', borderRadius: '8px', background: 'rgba(99, 102, 241, 0.1)', color: 'var(--accent-primary)' }}>
              <Server size={20} />
            </div>
          </div>
          <div style={{ fontSize: '2rem', fontWeight: 700 }}>{stats.servers}</div>
          <Link
            to="/servers"
            style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', fontSize: '0.8rem', color: 'var(--cyan-primary)', marginTop: '8px', textDecoration: 'none' }}
          >
            <span>Manage servers</span>
            <ArrowRight size={14} />
          </Link>
        </div>

        {/* Projects Card */}
        <div className="glass-panel" style={{ padding: '20px 24px' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
            <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', fontWeight: 500 }}>Active Projects</span>
            <div style={{ padding: '8px', borderRadius: '8px', background: 'rgba(6, 182, 212, 0.1)', color: 'var(--cyan-primary)' }}>
              <Layers size={20} />
            </div>
          </div>
          <div style={{ fontSize: '2rem', fontWeight: 700 }}>{stats.projects}</div>
          <Link
            to="/projects"
            style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', fontSize: '0.8rem', color: 'var(--cyan-primary)', marginTop: '8px', textDecoration: 'none' }}
          >
            <span>View projects</span>
            <ArrowRight size={14} />
          </Link>
        </div>

        {/* Applications Card */}
        <div className="glass-panel" style={{ padding: '20px 24px' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
            <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', fontWeight: 500 }}>Workloads & Apps</span>
            <div style={{ padding: '8px', borderRadius: '8px', background: 'rgba(16, 185, 129, 0.1)', color: 'var(--success)' }}>
              <Box size={20} />
            </div>
          </div>
          <div style={{ fontSize: '2rem', fontWeight: 700 }}>{stats.applications}</div>
          <Link
            to="/applications"
            style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', fontSize: '0.8rem', color: 'var(--cyan-primary)', marginTop: '8px', textDecoration: 'none' }}
          >
            <span>View applications</span>
            <ArrowRight size={14} />
          </Link>
        </div>

        {/* Engine Status Card */}
        <div className="glass-panel" style={{ padding: '20px 24px' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
            <span style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', fontWeight: 500 }}>Control Plane</span>
            <div style={{ padding: '8px', borderRadius: '8px', background: 'rgba(16, 185, 129, 0.1)', color: 'var(--success)' }}>
              <Database size={20} />
            </div>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span className="pulse-dot" style={{ color: 'var(--success)' }} />
            <span style={{ fontSize: '1.25rem', fontWeight: 600 }}>SQLite Healthy</span>
          </div>
          <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)', marginTop: '12px' }}>
            Queue: Loco BackgroundQueue
          </div>
        </div>
      </div>

      {/* Applications Overview Table */}
      <div className="glass-panel" style={{ padding: '24px', marginBottom: '32px' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '20px' }}>
          <div>
            <h2 style={{ fontSize: '1.25rem', fontWeight: 600 }}>Managed Applications</h2>
            <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>Real-time status of deployable workloads</p>
          </div>
          <Link to="/applications" className="btn btn-primary">
            <Box size={16} />
            <span>New Application</span>
          </Link>
        </div>

        {applications.length === 0 ? (
          <div style={{ textAlign: 'center', padding: '48px 0', color: 'var(--text-muted)' }}>
            <Box size={40} style={{ margin: '0 auto 12px', opacity: 0.4 }} />
            <p style={{ fontSize: '0.95rem' }}>No applications registered yet.</p>
            <p style={{ fontSize: '0.85rem', marginTop: '4px' }}>Connect a Git repository and start deploying.</p>
          </div>
        ) : (
          <div style={{ overflowX: 'auto' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', textAlign: 'left', fontSize: '0.875rem' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid var(--border-color)', color: 'var(--text-secondary)' }}>
                  <th style={{ padding: '12px 16px', fontWeight: 600 }}>Name</th>
                  <th style={{ padding: '12px 16px', fontWeight: 600 }}>Repository / Branch</th>
                  <th style={{ padding: '12px 16px', fontWeight: 600 }}>Build Type</th>
                  <th style={{ padding: '12px 16px', fontWeight: 600 }}>Port</th>
                  <th style={{ padding: '12px 16px', fontWeight: 600 }}>Status</th>
                  <th style={{ padding: '12px 16px', fontWeight: 600, textAlign: 'right' }}>Action</th>
                </tr>
              </thead>
              <tbody>
                {applications.map((app) => (
                  <tr
                    key={app.id}
                    style={{ borderBottom: '1px solid rgba(255, 255, 255, 0.04)', transition: 'background 0.15s ease' }}
                  >
                    <td style={{ padding: '14px 16px', fontWeight: 600 }}>
                      <Link to={`/applications/${app.id}`} style={{ color: '#fff', textDecoration: 'none' }}>
                        {app.name}
                      </Link>
                    </td>
                    <td style={{ padding: '14px 16px', color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}>
                      {app.git_repository.split('/').slice(-2).join('/')} : {app.git_branch}
                    </td>
                    <td style={{ padding: '14px 16px' }}>
                      <span style={{ textTransform: 'capitalize' }}>{app.build_type}</span>
                    </td>
                    <td style={{ padding: '14px 16px', fontFamily: 'var(--font-mono)' }}>
                      {app.published_port ? `${app.published_port}:${app.container_port}` : app.container_port}
                    </td>
                    <td style={{ padding: '14px 16px' }}>
                      <span
                        className={`badge ${
                          app.status === 'running'
                            ? 'badge-success'
                            : app.status === 'failed'
                            ? 'badge-danger'
                            : app.status === 'queued' || app.status === 'cloning' || app.status === 'building'
                            ? 'badge-building'
                            : 'badge-neutral'
                        }`}
                      >
                        {app.status}
                      </span>
                    </td>
                    <td style={{ padding: '14px 16px', textAlign: 'right' }}>
                      <Link to={`/applications/${app.id}`} className="btn btn-secondary" style={{ padding: '6px 12px', fontSize: '0.8rem' }}>
                        Manage
                      </Link>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
