import React, { useEffect, useState } from 'react';
import { Link } from 'react-router';
import { apiRequest } from '../api/client';
import { Box, Plus, GitBranch, Server, ArrowRight } from 'lucide-react';

export function Applications() {
  const [apps, setApps] = useState<any[]>([]);
  const [servers, setServers] = useState<any[]>([]);
  const [projects, setProjects] = useState<any[]>([]);
  const [showAddModal, setShowAddModal] = useState(false);

  const [form, setForm] = useState({
    name: '',
    slug: '',
    project_id: 1,
    environment_id: 1,
    server_id: 1,
    git_repository: '',
    git_branch: 'main',
    build_type: 'dockerfile',
    dockerfile_path: 'Dockerfile',
    docker_context: '.',
    container_port: 80,
    published_port: 8080,
    healthcheck_path: '/health',
  });

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

      if (serverList.length > 0) setForm((f) => ({ ...f, server_id: serverList[0].id }));
      if (projectList.length > 0) setForm((f) => ({ ...f, project_id: projectList[0].id }));
    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    loadData();
  }, []);

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    try {
      await apiRequest('/api/applications', {
        method: 'POST',
        body: JSON.stringify({
          ...form,
          project_id: Number(form.project_id),
          environment_id: Number(form.environment_id),
          server_id: Number(form.server_id),
          container_port: Number(form.container_port),
          published_port: form.published_port ? Number(form.published_port) : undefined,
        }),
      });
      setShowAddModal(false);
      await loadData();
    } catch (err: any) {
      alert(`Error creating application: ${err.message}`);
    }
  }

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

      {/* Add Application Modal */}
      {showAddModal && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            background: 'rgba(0, 0, 0, 0.75)',
            backdropFilter: 'blur(8px)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 50,
            padding: '20px',
          }}
        >
          <div className="glass-panel" style={{ width: '100%', maxWidth: '580px', padding: '32px', maxHeight: '90vh', overflowY: 'auto' }}>
            <h2 style={{ fontSize: '1.35rem', fontWeight: 700, marginBottom: '20px' }}>Deploy New Application</h2>
            <form onSubmit={handleCreate} style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
              <div>
                <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Application Name</label>
                <input
                  required
                  type="text"
                  placeholder="e.g. API Gateway"
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                  style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff' }}
                />
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                <div>
                  <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Project</label>
                  <select
                    value={form.project_id}
                    onChange={(e) => setForm({ ...form, project_id: Number(e.target.value) })}
                    style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: '#111827', border: '1px solid var(--border-color)', color: '#fff' }}
                  >
                    {projects.map((p) => (
                      <option key={p.id} value={p.id}>{p.name}</option>
                    ))}
                  </select>
                </div>

                <div>
                  <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Target Server</label>
                  <select
                    value={form.server_id}
                    onChange={(e) => setForm({ ...form, server_id: Number(e.target.value) })}
                    style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: '#111827', border: '1px solid var(--border-color)', color: '#fff' }}
                  >
                    {servers.map((s) => (
                      <option key={s.id} value={s.id}>{s.name} ({s.host})</option>
                    ))}
                  </select>
                </div>
              </div>

              <div>
                <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Git Repository URL</label>
                <input
                  required
                  type="text"
                  placeholder="https://github.com/org/repo.git"
                  value={form.git_repository}
                  onChange={(e) => setForm({ ...form, git_repository: e.target.value })}
                  style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff', fontFamily: 'var(--font-mono)' }}
                />
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                <div>
                  <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Git Branch</label>
                  <input
                    type="text"
                    value={form.git_branch}
                    onChange={(e) => setForm({ ...form, git_branch: e.target.value })}
                    style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff' }}
                  />
                </div>

                <div>
                  <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Build Type</label>
                  <select
                    value={form.build_type}
                    onChange={(e) => setForm({ ...form, build_type: e.target.value })}
                    style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: '#111827', border: '1px solid var(--border-color)', color: '#fff' }}
                  >
                    <option value="dockerfile">Dockerfile</option>
                    <option value="prebuilt_image">Prebuilt Image</option>
                  </select>
                </div>
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                <div>
                  <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Container Port</label>
                  <input
                    type="number"
                    value={form.container_port}
                    onChange={(e) => setForm({ ...form, container_port: Number(e.target.value) })}
                    style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff' }}
                  />
                </div>

                <div>
                  <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Published Host Port (Optional)</label>
                  <input
                    type="number"
                    value={form.published_port}
                    onChange={(e) => setForm({ ...form, published_port: Number(e.target.value) })}
                    style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff' }}
                  />
                </div>
              </div>

              <div>
                <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Healthcheck Path</label>
                <input
                  type="text"
                  placeholder="/health"
                  value={form.healthcheck_path}
                  onChange={(e) => setForm({ ...form, healthcheck_path: e.target.value })}
                  style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff' }}
                />
              </div>

              <div style={{ display: 'flex', gap: '12px', marginTop: '12px' }}>
                <button type="button" onClick={() => setShowAddModal(false)} className="btn btn-secondary" style={{ flex: 1 }}>
                  Cancel
                </button>
                <button type="submit" className="btn btn-primary" style={{ flex: 1 }}>
                  Create Application
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}
