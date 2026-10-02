import React, { useEffect, useState } from 'react';
import { apiRequest } from '../api/client';
import { Layers, Plus, Folder, ArrowRight } from 'lucide-react';

export function Projects() {
  const [projects, setProjects] = useState<any[]>([]);
  const [showAddModal, setShowAddModal] = useState(false);
  const [showAddEnvModal, setShowAddEnvModal] = useState<number | null>(null);

  const [projectForm, setProjectForm] = useState({
    name: '',
    slug: '',
    description: '',
  });

  const [envForm, setEnvForm] = useState({
    name: '',
    slug: '',
    description: '',
  });

  async function loadProjects() {
    try {
      const data = await apiRequest<any[]>('/api/projects');
      setProjects(data);
    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    loadProjects();
  }, []);

  async function handleCreateProject(e: React.FormEvent) {
    e.preventDefault();
    try {
      await apiRequest('/api/projects', {
        method: 'POST',
        body: JSON.stringify(projectForm),
      });
      setShowAddModal(false);
      setProjectForm({ name: '', slug: '', description: '' });
      await loadProjects();
    } catch (err: any) {
      alert(`Error creating project: ${err.message}`);
    }
  }

  async function handleCreateEnv(e: React.FormEvent, projectId: number) {
    e.preventDefault();
    try {
      await apiRequest(`/api/projects/${projectId}/environments`, {
        method: 'POST',
        body: JSON.stringify(envForm),
      });
      setShowAddEnvModal(null);
      setEnvForm({ name: '', slug: '', description: '' });
      alert('Environment created successfully!');
    } catch (err: any) {
      alert(`Error adding environment: ${err.message}`);
    }
  }

  return (
    <div style={{ maxWidth: '1200px', margin: '0 auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '32px' }}>
        <div>
          <h1 style={{ fontSize: '2rem', fontWeight: 700, letterSpacing: '-0.02em', marginBottom: '8px' }}>
            Projects
          </h1>
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem' }}>
            Logical organization for workloads, teams, and environments
          </p>
        </div>
        <button onClick={() => setShowAddModal(true)} className="btn btn-primary">
          <Plus size={16} />
          <span>New Project</span>
        </button>
      </div>

      {projects.length === 0 ? (
        <div className="glass-panel" style={{ padding: '64px 24px', textAlign: 'center', color: 'var(--text-muted)' }}>
          <Layers size={48} style={{ margin: '0 auto 16px', opacity: 0.4 }} />
          <h3 style={{ fontSize: '1.25rem', color: '#fff', marginBottom: '8px' }}>No Projects Found</h3>
          <p style={{ maxWidth: '400px', margin: '0 auto 24px', fontSize: '0.9rem' }}>
            Group applications into isolated projects and environments.
          </p>
          <button onClick={() => setShowAddModal(true)} className="btn btn-primary">
            <Plus size={16} />
            <span>Create First Project</span>
          </button>
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(360px, 1fr))', gap: '20px' }}>
          {projects.map((p) => (
            <div key={p.id} className="glass-panel" style={{ padding: '24px' }}>
              <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: '12px' }}>
                <div>
                  <h3 style={{ fontSize: '1.2rem', fontWeight: 600, color: '#fff' }}>{p.name}</h3>
                  <div style={{ fontFamily: 'var(--font-mono)', fontSize: '0.8rem', color: 'var(--cyan-primary)', marginTop: '2px' }}>
                    /{p.slug}
                  </div>
                </div>
                <div style={{ padding: '8px', borderRadius: '8px', background: 'rgba(6, 182, 212, 0.1)', color: 'var(--cyan-primary)' }}>
                  <Folder size={18} />
                </div>
              </div>

              <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginBottom: '20px', minHeight: '40px' }}>
                {p.description || 'No description provided.'}
              </p>

              <div style={{ display: 'flex', gap: '8px' }}>
                <button
                  onClick={() => setShowAddEnvModal(p.id)}
                  className="btn btn-secondary"
                  style={{ flex: 1, fontSize: '0.8rem', padding: '6px 12px' }}
                >
                  <Plus size={14} />
                  <span>Add Environment</span>
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Add Project Modal */}
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
          <div className="glass-panel" style={{ width: '100%', maxWidth: '480px', padding: '32px' }}>
            <h2 style={{ fontSize: '1.35rem', fontWeight: 700, marginBottom: '20px' }}>Create New Project</h2>
            <form onSubmit={handleCreateProject} style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
              <div>
                <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Project Name</label>
                <input
                  required
                  type="text"
                  placeholder="e.g. Core Platform"
                  value={projectForm.name}
                  onChange={(e) => setProjectForm({ ...projectForm, name: e.target.value })}
                  style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff' }}
                />
              </div>

              <div>
                <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Slug (Optional)</label>
                <input
                  type="text"
                  placeholder="core-platform"
                  value={projectForm.slug}
                  onChange={(e) => setProjectForm({ ...projectForm, slug: e.target.value })}
                  style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff' }}
                />
              </div>

              <div>
                <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Description</label>
                <textarea
                  rows={3}
                  placeholder="Brief description of workloads in this project"
                  value={projectForm.description}
                  onChange={(e) => setProjectForm({ ...projectForm, description: e.target.value })}
                  style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff' }}
                />
              </div>

              <div style={{ display: 'flex', gap: '12px', marginTop: '12px' }}>
                <button type="button" onClick={() => setShowAddModal(false)} className="btn btn-secondary" style={{ flex: 1 }}>
                  Cancel
                </button>
                <button type="submit" className="btn btn-primary" style={{ flex: 1 }}>
                  Create Project
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Add Environment Modal */}
      {showAddEnvModal && (
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
          <div className="glass-panel" style={{ width: '100%', maxWidth: '440px', padding: '32px' }}>
            <h2 style={{ fontSize: '1.35rem', fontWeight: 700, marginBottom: '20px' }}>Add Environment</h2>
            <form onSubmit={(e) => handleCreateEnv(e, showAddEnvModal)} style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
              <div>
                <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Environment Name</label>
                <input
                  required
                  type="text"
                  placeholder="e.g. production, staging, dev"
                  value={envForm.name}
                  onChange={(e) => setEnvForm({ ...envForm, name: e.target.value })}
                  style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff' }}
                />
              </div>

              <div style={{ display: 'flex', gap: '12px', marginTop: '12px' }}>
                <button type="button" onClick={() => setShowAddEnvModal(null)} className="btn btn-secondary" style={{ flex: 1 }}>
                  Cancel
                </button>
                <button type="submit" className="btn btn-primary" style={{ flex: 1 }}>
                  Save Environment
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}
