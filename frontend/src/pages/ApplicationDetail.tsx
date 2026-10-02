import React, { useEffect, useState, useRef } from 'react';
import { useParams, Link } from 'react-router';
import { apiRequest } from '../api/client';
import {
  ArrowLeft,
  Rocket,
  Play,
  Square,
  RotateCw,
  Terminal,
  Key,
  Globe,
  GitBranch,
  Server,
  Layers,
  CheckCircle,
  XCircle,
  Clock,
  AlertTriangle,
  Trash2,
  Plus,
  RefreshCw,
} from 'lucide-react';

export function ApplicationDetail() {
  const { id } = useParams<{ id: string }>();
  const [app, setApp] = useState<any>(null);
  const [activeTab, setActiveTab] = useState<'overview' | 'deployments' | 'logs' | 'env' | 'domains'>('overview');
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);

  // Deployments state
  const [deployments, setDeployments] = useState<any[]>([]);
  const [activeDeploymentId, setActiveDeploymentId] = useState<number | null>(null);
  const [deploymentLogs, setDeploymentLogs] = useState<any[]>([]);

  // Environment variables state
  const [envVars, setEnvVars] = useState<any[]>([]);
  const [newEnvKey, setNewEnvKey] = useState('');
  const [newEnvValue, setNewEnvValue] = useState('');
  const [newEnvSecret, setNewEnvSecret] = useState(false);

  // Domains state
  const [domains, setDomains] = useState<any[]>([]);
  const [newHostname, setNewHostname] = useState('');
  const [newDomainPort, setNewDomainPort] = useState(80);
  const [newHttps, setNewHttps] = useState(true);

  // Container live status & logs
  const [containerStatus, setContainerStatus] = useState<any>(null);
  const [containerLogs, setContainerLogs] = useState<string>('');

  const logEndRef = useRef<HTMLDivElement | null>(null);

  async function loadAppData() {
    if (!id) return;
    try {
      const data = await apiRequest<any>(`/api/applications/${id}`);
      setApp(data);
    } catch (e) {
      console.error('Failed to load application:', e);
    } finally {
      setLoading(false);
    }
  }

  async function loadEnvVars() {
    if (!id) return;
    try {
      const data = await apiRequest<any[]>(`/api/applications/${id}/environment`);
      setEnvVars(data);
    } catch (e) {
      console.error(e);
    }
  }

  async function loadDomains() {
    if (!id) return;
    try {
      const data = await apiRequest<any[]>(`/api/applications/${id}/domains`);
      setDomains(data);
    } catch (e) {
      console.error(e);
    }
  }

  async function loadDeployments() {
    if (!id) return;
    try {
      const allDeps = await apiRequest<any[]>('/api/deployments').catch(() => []);
      const appDeps = allDeps.filter((d: any) => d.application_id === Number(id));
      setDeployments(appDeps);
      if (appDeps.length > 0 && !activeDeploymentId) {
        setActiveDeploymentId(appDeps[0].id);
      }
    } catch (e) {
      console.error(e);
    }
  }

  async function loadDeploymentLogs(depId: number) {
    try {
      const logs = await apiRequest<any[]>(`/api/deployments/${depId}/logs`);
      setDeploymentLogs(logs);
    } catch (e) {
      console.error(e);
    }
  }

  async function loadContainerStatus() {
    if (!id) return;
    try {
      const res = await apiRequest<any>(`/api/applications/${id}/status`);
      setContainerStatus(res);
    } catch (e) {
      console.error(e);
    }
  }

  async function loadContainerLogs() {
    if (!id) return;
    try {
      const res = await apiRequest<any>(`/api/applications/${id}/logs`);
      setContainerLogs(res.logs || 'No logs found.');
    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    loadAppData();
    loadDeployments();
    loadEnvVars();
    loadDomains();
    loadContainerStatus();
  }, [id]);

  useEffect(() => {
    if (activeDeploymentId && activeTab === 'deployments') {
      loadDeploymentLogs(activeDeploymentId);
      const interval = setInterval(() => {
        loadDeploymentLogs(activeDeploymentId);
        loadDeployments();
      }, 3000);
      return () => clearInterval(interval);
    }
  }, [activeDeploymentId, activeTab]);

  useEffect(() => {
    if (activeTab === 'logs') {
      loadContainerLogs();
      const interval = setInterval(() => {
        loadContainerLogs();
      }, 4000);
      return () => clearInterval(interval);
    }
  }, [activeTab, id]);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [deploymentLogs, containerLogs]);

  async function handleDeploy() {
    if (!id) return;
    setActionLoading(true);
    try {
      const res = await apiRequest<any>(`/api/applications/${id}/deploy`, { method: 'POST' });
      alert(`Deployment #${res.deployment_id} queued successfully!`);
      setActiveTab('deployments');
      await loadDeployments();
      if (res.deployment_id) {
        setActiveDeploymentId(res.deployment_id);
      }
    } catch (e: any) {
      alert(`Deployment failed: ${e.message}`);
    } finally {
      setActionLoading(false);
    }
  }

  async function handleLifecycleAction(action: 'start' | 'stop' | 'restart') {
    if (!id) return;
    setActionLoading(true);
    try {
      await apiRequest(`/api/applications/${id}/${action}`, { method: 'POST' });
      await loadAppData();
      await loadContainerStatus();
    } catch (e: any) {
      alert(`Action ${action} failed: ${e.message}`);
    } finally {
      setActionLoading(false);
    }
  }

  async function handleAddEnv(e: React.FormEvent) {
    e.preventDefault();
    if (!id || !newEnvKey) return;
    try {
      await apiRequest(`/api/applications/${id}/environment`, {
        method: 'POST',
        body: JSON.stringify({
          key: newEnvKey,
          value: newEnvValue,
          is_secret: newEnvSecret,
        }),
      });
      setNewEnvKey('');
      setNewEnvValue('');
      setNewEnvSecret(false);
      await loadEnvVars();
    } catch (e: any) {
      alert(`Error saving environment variable: ${e.message}`);
    }
  }

  async function handleDeleteEnv(key: string) {
    if (!id || !confirm(`Delete environment variable "${key}"?`)) return;
    try {
      await apiRequest(`/api/applications/${id}/environment/${key}`, { method: 'DELETE' });
      await loadEnvVars();
    } catch (e: any) {
      alert(`Failed to delete: ${e.message}`);
    }
  }

  async function handleAddDomain(e: React.FormEvent) {
    e.preventDefault();
    if (!id || !newHostname) return;
    try {
      await apiRequest(`/api/applications/${id}/domains`, {
        method: 'POST',
        body: JSON.stringify({
          hostname: newHostname,
          port: Number(newDomainPort),
          https_enabled: newHttps,
        }),
      });
      setNewHostname('');
      await loadDomains();
    } catch (e: any) {
      alert(`Error adding domain: ${e.message}`);
    }
  }

  async function handleDeleteDomain(domainId: number) {
    if (!id || !confirm('Remove this domain mapping?')) return;
    try {
      await apiRequest(`/api/applications/${id}/domains/${domainId}`, { method: 'DELETE' });
      await loadDomains();
    } catch (e: any) {
      alert(`Failed to remove domain: ${e.message}`);
    }
  }

  if (loading) {
    return (
      <div style={{ textAlign: 'center', padding: '100px 0', color: 'var(--text-secondary)' }}>
        Loading application details...
      </div>
    );
  }

  if (!app) {
    return (
      <div style={{ textAlign: 'center', padding: '100px 0' }}>
        <h2 style={{ marginBottom: '16px' }}>Application not found</h2>
        <Link to="/applications" className="btn btn-secondary">
          <ArrowLeft size={16} /> Back to Applications
        </Link>
      </div>
    );
  }

  const isRunning = app.status === 'running';

  return (
    <div style={{ maxWidth: '1200px', margin: '0 auto' }}>
      {/* Top Breadcrumb & Actions Bar */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '24px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <Link to="/applications" className="btn btn-secondary" style={{ padding: '8px 12px' }}>
            <ArrowLeft size={16} /> Back
          </Link>
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <h1 style={{ fontSize: '1.75rem', fontWeight: 700, letterSpacing: '-0.02em' }}>{app.name}</h1>
              <span className={`status-badge status-${app.status}`}>{app.status}</span>
            </div>
            <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>Slug: {app.slug}</p>
          </div>
        </div>

        {/* Action Buttons */}
        <div style={{ display: 'flex', gap: '8px' }}>
          <button
            onClick={handleDeploy}
            disabled={actionLoading}
            className="btn btn-primary"
            style={{ boxShadow: '0 0 16px rgba(99, 102, 241, 0.4)' }}
          >
            <Rocket size={16} />
            <span>Deploy</span>
          </button>
          {isRunning ? (
            <>
              <button
                onClick={() => handleLifecycleAction('restart')}
                disabled={actionLoading}
                className="btn btn-secondary"
              >
                <RotateCw size={16} /> Restart
              </button>
              <button
                onClick={() => handleLifecycleAction('stop')}
                disabled={actionLoading}
                className="btn btn-danger"
              >
                <Square size={16} /> Stop
              </button>
            </>
          ) : (
            <button
              onClick={() => handleLifecycleAction('start')}
              disabled={actionLoading}
              className="btn btn-secondary"
            >
              <Play size={16} /> Start
            </button>
          )}
        </div>
      </div>

      {/* Tabs */}
      <div style={{ display: 'flex', gap: '8px', borderBottom: '1px solid var(--border-color)', marginBottom: '24px' }}>
        {[
          { key: 'overview', label: 'Overview', icon: Layers },
          { key: 'deployments', label: `Deployments (${deployments.length})`, icon: Rocket },
          { key: 'logs', label: 'Runtime Logs', icon: Terminal },
          { key: 'env', label: `Environment (${envVars.length})`, icon: Key },
          { key: 'domains', label: `Domains (${domains.length})`, icon: Globe },
        ].map((t) => {
          const Icon = t.icon;
          const isActive = activeTab === t.key;
          return (
            <button
              key={t.key}
              onClick={() => setActiveTab(t.key as any)}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                padding: '10px 16px',
                background: 'transparent',
                border: 'none',
                borderBottom: isActive ? '2px solid var(--indigo-primary)' : '2px solid transparent',
                color: isActive ? '#fff' : 'var(--text-secondary)',
                fontWeight: isActive ? 600 : 500,
                cursor: 'pointer',
                transition: 'all 0.15s ease',
              }}
            >
              <Icon size={16} />
              <span>{t.label}</span>
            </button>
          );
        })}
      </div>

      {/* TAB 1: OVERVIEW */}
      {activeTab === 'overview' && (
        <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: '24px' }}>
          <div className="card" style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
            <h3 style={{ fontSize: '1.1rem', fontWeight: 600 }}>Configuration & Build Spec</h3>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
              <div>
                <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>
                  Git Repository
                </label>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginTop: '4px', fontWeight: 500 }}>
                  <GitBranch size={16} color="var(--indigo-primary)" />
                  <span>{app.git_repository || 'None (Prebuilt Image)'}</span>
                </div>
              </div>
              <div>
                <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>
                  Branch
                </label>
                <div style={{ marginTop: '4px', fontWeight: 500 }}>{app.git_branch || 'main'}</div>
              </div>
              <div>
                <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>
                  Build Type
                </label>
                <div style={{ marginTop: '4px', fontWeight: 500 }}>{app.build_type}</div>
              </div>
              <div>
                <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>
                  Dockerfile Path
                </label>
                <div style={{ marginTop: '4px', fontWeight: 500 }}>{app.dockerfile_path || 'Dockerfile'}</div>
              </div>
              <div>
                <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>
                  Internal Container Port
                </label>
                <div style={{ marginTop: '4px', fontWeight: 500 }}>{app.container_port}</div>
              </div>
              <div>
                <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>
                  Host Published Port
                </label>
                <div style={{ marginTop: '4px', fontWeight: 500 }}>{app.published_port || 'Auto (Dynamic)'}</div>
              </div>
              <div>
                <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>
                  Healthcheck Path
                </label>
                <div style={{ marginTop: '4px', fontWeight: 500 }}>{app.healthcheck_path || '/'}</div>
              </div>
              <div>
                <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>
                  Container Name
                </label>
                <div style={{ marginTop: '4px', fontFamily: 'monospace', fontSize: '0.9rem' }}>
                  {app.container_name || `moonships-app-${app.id}`}
                </div>
              </div>
            </div>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
            <div className="card">
              <h3 style={{ fontSize: '1.1rem', fontWeight: 600, marginBottom: '16px' }}>Deployment Target</h3>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                  <Server size={18} color="var(--cyan-primary)" />
                  <div>
                    <div style={{ fontWeight: 600, fontSize: '0.95rem' }}>Server #{app.server_id}</div>
                    <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>Docker Host Control</div>
                  </div>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                  <Layers size={18} color="var(--indigo-primary)" />
                  <div>
                    <div style={{ fontWeight: 600, fontSize: '0.95rem' }}>Project #{app.project_id}</div>
                    <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>Environment #{app.environment_id}</div>
                  </div>
                </div>
              </div>
            </div>

            <div className="card">
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
                <h3 style={{ fontSize: '1rem', fontWeight: 600 }}>Container Health</h3>
                <button onClick={loadContainerStatus} className="btn btn-secondary" style={{ padding: '4px 8px' }}>
                  <RefreshCw size={12} />
                </button>
              </div>
              <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', display: 'flex', flexDirection: 'column', gap: '6px' }}>
                <div>State: <span style={{ color: '#fff', fontWeight: 600 }}>{containerStatus?.status || app.status}</span></div>
                <div>Docker Engine: <span style={{ color: 'var(--success)' }}>Connected</span></div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* TAB 2: DEPLOYMENTS & LIVE DEPLOYMENT LOGS */}
      {activeTab === 'deployments' && (
        <div style={{ display: 'grid', gridTemplateColumns: '320px 1fr', gap: '24px' }}>
          {/* Deployment List Sidebar */}
          <div className="card" style={{ padding: '16px', maxHeight: '600px', overflowY: 'auto' }}>
            <h3 style={{ fontSize: '1rem', fontWeight: 600, marginBottom: '12px' }}>Deployment History</h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              {deployments.map((d) => {
                const isSelected = activeDeploymentId === d.id;
                return (
                  <div
                    key={d.id}
                    onClick={() => setActiveDeploymentId(d.id)}
                    style={{
                      padding: '10px 12px',
                      borderRadius: '8px',
                      background: isSelected ? 'var(--accent-glow)' : 'rgba(255, 255, 255, 0.02)',
                      border: isSelected ? '1px solid var(--indigo-primary)' : '1px solid var(--border-color)',
                      cursor: 'pointer',
                      transition: 'all 0.15s ease',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '4px' }}>
                      <span style={{ fontWeight: 600, fontSize: '0.9rem' }}>#{d.id}</span>
                      <span className={`status-badge status-${d.status}`}>{d.status}</span>
                    </div>
                    <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
                      {d.commit_hash ? `Commit ${d.commit_hash.substring(0, 7)}` : 'Manual Trigger'}
                    </div>
                    <div style={{ fontSize: '0.7rem', color: 'var(--text-muted)', marginTop: '4px' }}>
                      {new Date(d.created_at || d.queued_at).toLocaleTimeString()}
                    </div>
                  </div>
                );
              })}
              {deployments.length === 0 && (
                <div style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', textAlign: 'center', padding: '24px 0' }}>
                  No deployments yet. Click "Deploy" to begin.
                </div>
              )}
            </div>
          </div>

          {/* Log Stream Terminal */}
          <div className="card" style={{ padding: '20px', display: 'flex', flexDirection: 'column', height: '600px' }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Terminal size={18} color="var(--cyan-primary)" />
                <span style={{ fontWeight: 600 }}>
                  Deployment Logs {activeDeploymentId ? `(#${activeDeploymentId})` : ''}
                </span>
              </div>
              <button
                onClick={() => activeDeploymentId && loadDeploymentLogs(activeDeploymentId)}
                className="btn btn-secondary"
                style={{ padding: '4px 10px', fontSize: '0.75rem' }}
              >
                <RefreshCw size={12} /> Refresh
              </button>
            </div>

            <div
              style={{
                flex: 1,
                background: '#090d16',
                border: '1px solid rgba(255, 255, 255, 0.08)',
                borderRadius: '8px',
                padding: '16px',
                fontFamily: 'monospace',
                fontSize: '0.85rem',
                color: '#e2e8f0',
                overflowY: 'auto',
                lineHeight: 1.5,
              }}
            >
              {deploymentLogs.map((log) => {
                const isErr = log.stream === 'stderr';
                const isSys = log.stream === 'system';
                return (
                  <div key={log.id || log.sequence} style={{ marginBottom: '4px', wordBreak: 'break-all' }}>
                    <span style={{ color: 'var(--text-muted)', marginRight: '8px' }}>
                      [{new Date(log.created_at).toLocaleTimeString()}]
                    </span>
                    <span
                      style={{
                        color: isErr ? 'var(--danger)' : isSys ? 'var(--cyan-primary)' : '#e2e8f0',
                      }}
                    >
                      {log.message}
                    </span>
                  </div>
                );
              })}
              {deploymentLogs.length === 0 && (
                <div style={{ color: 'var(--text-muted)' }}>Waiting for deployment worker stream...</div>
              )}
              <div ref={logEndRef} />
            </div>
          </div>
        </div>
      )}

      {/* TAB 3: CONTAINER RUNTIME LOGS */}
      {activeTab === 'logs' && (
        <div className="card" style={{ padding: '20px', display: 'flex', flexDirection: 'column', height: '600px' }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Terminal size={18} color="var(--success)" />
              <span style={{ fontWeight: 600 }}>Live Docker Container Stdout/Stderr</span>
            </div>
            <button
              onClick={loadContainerLogs}
              className="btn btn-secondary"
              style={{ padding: '4px 10px', fontSize: '0.75rem' }}
            >
              <RefreshCw size={12} /> Refresh
            </button>
          </div>

          <pre
            style={{
              flex: 1,
              background: '#090d16',
              border: '1px solid rgba(255, 255, 255, 0.08)',
              borderRadius: '8px',
              padding: '16px',
              fontFamily: 'monospace',
              fontSize: '0.85rem',
              color: '#34d399',
              overflowY: 'auto',
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
              margin: 0,
            }}
          >
            {containerLogs || 'No logs received from container runtime.'}
          </pre>
        </div>
      )}

      {/* TAB 4: ENVIRONMENT VARIABLES */}
      {activeTab === 'env' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
          <div className="card">
            <h3 style={{ fontSize: '1.1rem', fontWeight: 600, marginBottom: '8px' }}>Environment Variables & Secrets</h3>
            <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginBottom: '20px' }}>
              Secrets are encrypted using AES-256-GCM and masked in API responses. Injected into runtime container.
            </p>

            <form
              onSubmit={handleAddEnv}
              style={{
                display: 'grid',
                gridTemplateColumns: '1fr 1fr auto auto',
                gap: '12px',
                alignItems: 'center',
                marginBottom: '24px',
                padding: '16px',
                background: 'rgba(255, 255, 255, 0.02)',
                borderRadius: '8px',
                border: '1px solid var(--border-color)',
              }}
            >
              <input
                className="input-field"
                placeholder="VARIABLE_NAME (e.g. DATABASE_URL)"
                value={newEnvKey}
                onChange={(e) => setNewEnvKey(e.target.value.toUpperCase())}
                required
              />
              <input
                className="input-field"
                placeholder="Value"
                value={newEnvValue}
                onChange={(e) => setNewEnvValue(e.target.value)}
                required
              />
              <label style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '0.85rem', cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={newEnvSecret}
                  onChange={(e) => setNewEnvSecret(e.target.checked)}
                />
                Secret
              </label>
              <button type="submit" className="btn btn-primary" style={{ padding: '8px 16px' }}>
                <Plus size={16} /> Add Variable
              </button>
            </form>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              {envVars.map((env) => (
                <div
                  key={env.id || env.key}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '10px 16px',
                    borderRadius: '8px',
                    background: 'rgba(255, 255, 255, 0.02)',
                    border: '1px solid var(--border-color)',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
                    <code style={{ fontWeight: 600, color: 'var(--cyan-primary)' }}>{env.key}</code>
                    <span style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', fontFamily: 'monospace' }}>
                      {env.encrypted_value || '••••••••'}
                    </span>
                    {env.is_secret && (
                      <span className="status-badge" style={{ background: 'rgba(239, 68, 68, 0.15)', color: '#ef4444' }}>
                        Encrypted Secret
                      </span>
                    )}
                  </div>
                  <button
                    onClick={() => handleDeleteEnv(env.key)}
                    className="btn btn-danger"
                    style={{ padding: '6px 10px' }}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}
              {envVars.length === 0 && (
                <div style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: '24px 0', fontSize: '0.9rem' }}>
                  No environment variables configured.
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* TAB 5: DOMAINS & TRAEFIK ROUTING */}
      {activeTab === 'domains' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
          <div className="card">
            <h3 style={{ fontSize: '1.1rem', fontWeight: 600, marginBottom: '8px' }}>Custom Domains & Routing</h3>
            <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginBottom: '20px' }}>
              Configured hostnames generate Traefik dynamic labels routing incoming HTTP/HTTPS traffic to this container.
            </p>

            <form
              onSubmit={handleAddDomain}
              style={{
                display: 'grid',
                gridTemplateColumns: '2fr 1fr auto auto',
                gap: '12px',
                alignItems: 'center',
                marginBottom: '24px',
                padding: '16px',
                background: 'rgba(255, 255, 255, 0.02)',
                borderRadius: '8px',
                border: '1px solid var(--border-color)',
              }}
            >
              <input
                className="input-field"
                placeholder="app.example.com"
                value={newHostname}
                onChange={(e) => setNewHostname(e.target.value)}
                required
              />
              <input
                className="input-field"
                type="number"
                placeholder="Port"
                value={newDomainPort}
                onChange={(e) => setNewDomainPort(Number(e.target.value))}
                required
              />
              <label style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '0.85rem', cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={newHttps}
                  onChange={(e) => setNewHttps(e.target.checked)}
                />
                HTTPS (Traefik)
              </label>
              <button type="submit" className="btn btn-primary" style={{ padding: '8px 16px' }}>
                <Plus size={16} /> Map Domain
              </button>
            </form>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              {domains.map((dom) => (
                <div
                  key={dom.id}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '10px 16px',
                    borderRadius: '8px',
                    background: 'rgba(255, 255, 255, 0.02)',
                    border: '1px solid var(--border-color)',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
                    <Globe size={18} color="var(--indigo-primary)" />
                    <span style={{ fontWeight: 600 }}>{dom.hostname}</span>
                    <span style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>Port: {dom.port}</span>
                    {dom.https_enabled ? (
                      <span className="status-badge status-running">HTTPS Enabled</span>
                    ) : (
                      <span className="status-badge">HTTP Only</span>
                    )}
                  </div>
                  <button
                    onClick={() => handleDeleteDomain(dom.id)}
                    className="btn btn-danger"
                    style={{ padding: '6px 10px' }}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}
              {domains.length === 0 && (
                <div style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: '24px 0', fontSize: '0.9rem' }}>
                  No domains mapped yet.
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
