import React, { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router';
import { apiRequest } from '../api/client';
import { Box, Plus, GitBranch, Server, ArrowRight } from 'lucide-react';
import { ApplicationOnboardingModal } from '../components/ApplicationOnboardingModal';

export function Applications() {
  const navigate = useNavigate();
  const [apps, setApps] = useState<any[]>([]);
  const [servers, setServers] = useState<any[]>([]);
  const [projects, setProjects] = useState<any[]>([]);
  const [showAddModal, setShowAddModal] = useState(false);

  async function loadData() {
    try {
      const [appList, serverList, projectList] = await Promise.all([
        apiRequest<any[]>('/api/applications'),
        apiRequest<any[]>('/api/servers').catch(() => []),
        apiRequest<any[]>('/api/projects').catch(() => []),
      ]);
      setApps(appList);
      setServers(serverList);
      setProjects(projectList);

    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    loadData();
  }, []);

  return (
    <div style={{ maxWidth: '1200px', margin: '0 auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '32px' }}>
        <div>
          <h1 style={{ fontSize: '2rem', fontWeight: 700, letterSpacing: '-0.02em', marginBottom: '8px' }}>
            Applications
          </h1>
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem' }}>
            Deployable software services built from Git or container images
          </p>
        </div>
        <button onClick={() => setShowAddModal(true)} className="btn btn-primary">
          <Plus size={16} />
          <span>New Application</span>
        </button>
      </div>

      {apps.length === 0 ? (
        <div className="glass-panel" style={{ padding: '64px 24px', textAlign: 'center', color: 'var(--text-muted)' }}>
          <Box size={48} style={{ margin: '0 auto 16px', opacity: 0.4 }} />
          <h3 style={{ fontSize: '1.25rem', color: '#fff', marginBottom: '8px' }}>No Applications Yet</h3>
          <p style={{ maxWidth: '400px', margin: '0 auto 24px', fontSize: '0.9rem' }}>
            Create an application to link your Git repository and trigger continuous deployments.
          </p>
          <button onClick={() => setShowAddModal(true)} className="btn btn-primary">
            <Plus size={16} />
            <span>Create First Application</span>
          </button>
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(360px, 1fr))', gap: '20px' }}>
          {apps.map((app) => (
            <div key={app.id} className="glass-panel" style={{ padding: '24px' }}>
              <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: '16px' }}>
                <div>
                  <h3 style={{ fontSize: '1.2rem', fontWeight: 600, color: '#fff' }}>{app.name}</h3>
                  <div style={{ fontFamily: 'var(--font-mono)', fontSize: '0.8rem', color: 'var(--cyan-primary)', marginTop: '2px' }}>
                    /{app.slug}
                  </div>
                </div>
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
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', fontSize: '0.85rem', color: 'var(--text-secondary)', marginBottom: '20px' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <GitBranch size={15} color="var(--text-muted)" />
                  <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.8rem' }}>
                    {app.git_repository.split('/').slice(-2).join('/')} : {app.git_branch}
                  </span>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <Server size={15} color="var(--text-muted)" />
                  <span>
                    Port: {app.published_port ? `${app.published_port} -> ${app.container_port}` : app.container_port}
                  </span>
                </div>
              </div>

              <Link
                to={`/applications/${app.id}`}
                className="btn btn-secondary"
                style={{ width: '100%', justifyContent: 'center' }}
              >
                <span>Control Panel & Logs</span>
                <ArrowRight size={14} />
              </Link>
            </div>
          ))}
        </div>
      )}

      {showAddModal && (
        <ApplicationOnboardingModal
          projects={projects}
          servers={servers}
          onCancel={() => {
            setShowAddModal(false);
            void loadData();
          }}
          onComplete={(applicationId) => {
            setShowAddModal(false);
            void loadData();
            navigate(`/applications/${applicationId}`);
          }}
        />
      )}
    </div>
  );
}
