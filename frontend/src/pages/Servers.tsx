import React, { useEffect, useState } from 'react';
import { apiRequest } from '../api/client';
import {
  Server,
  Plus,
  Play,
  CheckCircle,
  XCircle,
  AlertTriangle,
  Cpu,
  HardDrive,
  RefreshCw,
  Key,
} from 'lucide-react';

export function Servers() {
  const [servers, setServers] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [showAddModal, setShowAddModal] = useState(false);
  const [preflightReport, setPreflightReport] = useState<any>(null);
  const [inspectingId, setInspectingId] = useState<number | null>(null);

  const [form, setForm] = useState({
    name: '',
    host: '',
    port: 22,
    username: 'root',
    authentication_type: 'ssh_key',
    private_key: '',
    known_host_fingerprint: '',
  });

  async function loadServers() {
    try {
      const data = await apiRequest<any[]>('/api/servers');
      setServers(data);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadServers();
  }, []);

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    try {
      await apiRequest('/api/servers', {
        method: 'POST',
        body: JSON.stringify({
          ...form,
          port: Number(form.port),
          private_key: form.private_key || undefined,
          known_host_fingerprint: form.known_host_fingerprint || undefined,
        }),
      });
      setShowAddModal(false);
      setForm({
        name: '',
        host: '',
        port: 22,
        username: 'root',
        authentication_type: 'ssh_key',
        private_key: '',
        known_host_fingerprint: '',
      });
      await loadServers();
    } catch (err: any) {
      alert(`Failed to add server: ${err.message}`);
    }
  }

  async function handleTest(id: number) {
    try {
      const res = await apiRequest<{ connected: boolean }>(`/api/servers/${id}/test-connection`, {
        method: 'POST',
      });
      alert(res.connected ? 'SSH Connection Successful!' : 'Connection Failed.');
      await loadServers();
    } catch (err: any) {
      alert(`Test connection error: ${err.message}`);
    }
  }

  async function handlePreflight(id: number) {
    setInspectingId(id);
    try {
      const report = await apiRequest(`/api/servers/${id}/preflight`, {
        method: 'POST',
      });
      setPreflightReport(report);
      await loadServers();
    } catch (err: any) {
      alert(`Preflight failed: ${err.message}`);
    } finally {
      setInspectingId(null);
    }
  }

  return (
    <div style={{ maxWidth: '1200px', margin: '0 auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '32px' }}>
        <div>
          <h1 style={{ fontSize: '2rem', fontWeight: 700, letterSpacing: '-0.02em', marginBottom: '8px' }}>
            Managed Servers
          </h1>
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem' }}>
            Linux hosts managed by Moonships via structured SSH and remote Docker engine
          </p>
        </div>
        <button onClick={() => setShowAddModal(true)} className="btn btn-primary">
          <Plus size={16} />
          <span>Add Server</span>
        </button>
      </div>

      {servers.length === 0 ? (
        <div className="glass-panel" style={{ padding: '64px 24px', textAlign: 'center', color: 'var(--text-muted)' }}>
          <Server size={48} style={{ margin: '0 auto 16px', opacity: 0.4 }} />
          <h3 style={{ fontSize: '1.25rem', color: '#fff', marginBottom: '8px' }}>No Servers Configured</h3>
          <p style={{ maxWidth: '400px', margin: '0 auto 24px', fontSize: '0.9rem' }}>
            Connect your first remote Linux VPS or bare metal node by providing its SSH credentials.
          </p>
          <button onClick={() => setShowAddModal(true)} className="btn btn-primary">
            <Plus size={16} />
            <span>Connect First Server</span>
          </button>
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(360px, 1fr))', gap: '20px' }}>
          {servers.map((s) => (
            <div key={s.id} className="glass-panel" style={{ padding: '24px' }}>
              <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: '16px' }}>
                <div>
                  <h3 style={{ fontSize: '1.2rem', fontWeight: 600, color: '#fff' }}>{s.name}</h3>
                  <div style={{ fontFamily: 'var(--font-mono)', fontSize: '0.85rem', color: 'var(--text-secondary)', marginTop: '4px' }}>
                    {s.username}@{s.host}:{s.port}
                  </div>
                </div>
                <span
                  className={`badge ${
                    s.status === 'online'
                      ? 'badge-success'
                      : s.status === 'error'
                      ? 'badge-danger'
                      : 'badge-neutral'
                  }`}
                >
                  {s.status}
                </span>
              </div>

              <div style={{ display: 'flex', gap: '16px', marginBottom: '20px', fontSize: '0.8rem', color: 'var(--text-muted)' }}>
                <div>Auth: {s.authentication_type}</div>
                <div>Last Seen: {s.last_seen_at ? new Date(s.last_seen_at).toLocaleTimeString() : 'Never'}</div>
              </div>

              <div style={{ display: 'flex', gap: '10px' }}>
                <button
                  onClick={() => handleTest(s.id)}
                  className="btn btn-secondary"
                  style={{ flex: 1, fontSize: '0.8rem', padding: '6px 12px' }}
                >
                  <RefreshCw size={14} />
                  <span>Ping SSH</span>
                </button>
                <button
                  onClick={() => handlePreflight(s.id)}
                  disabled={inspectingId === s.id}
                  className="btn btn-primary"
                  style={{ flex: 1, fontSize: '0.8rem', padding: '6px 12px' }}
                >
                  <Play size={14} />
                  <span>{inspectingId === s.id ? 'Inspecting...' : 'Preflight'}</span>
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Preflight Modal */}
      {preflightReport && (
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
          <div className="glass-panel" style={{ width: '100%', maxWidth: '540px', padding: '32px' }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '24px' }}>
              <h2 style={{ fontSize: '1.35rem', fontWeight: 700 }}>Server Preflight Report</h2>
              <button
                onClick={() => setPreflightReport(null)}
                style={{ background: 'transparent', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', fontSize: '1.25rem' }}
              >
                ✕
              </button>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', marginBottom: '24px' }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px', background: 'rgba(255, 255, 255, 0.03)', borderRadius: '8px' }}>
                <span>SSH Connectivity</span>
                {preflightReport.ssh_connected ? (
                  <span style={{ color: 'var(--success)', display: 'flex', alignItems: 'center', gap: '6px', fontSize: '0.85rem' }}>
                    <CheckCircle size={16} /> Connected
                  </span>
                ) : (
                  <span style={{ color: 'var(--danger)', display: 'flex', alignItems: 'center', gap: '6px', fontSize: '0.85rem' }}>
                    <XCircle size={16} /> Failed
                  </span>
                )}
              </div>

              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px', background: 'rgba(255, 255, 255, 0.03)', borderRadius: '8px' }}>
                <span>Docker Engine</span>
                <span style={{ color: preflightReport.docker_running ? 'var(--success)' : 'var(--danger)', fontSize: '0.85rem' }}>
                  {preflightReport.docker_version || 'Not Detected'}
                </span>
              </div>

              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px', background: 'rgba(255, 255, 255, 0.03)', borderRadius: '8px' }}>
                <span>Disk Available</span>
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.85rem' }}>
                  {preflightReport.disk_available_gb ? `${preflightReport.disk_available_gb} GB` : 'N/A'}
                </span>
              </div>

              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px', background: 'rgba(255, 255, 255, 0.03)', borderRadius: '8px' }}>
                <span>Memory Available</span>
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.85rem' }}>
                  {preflightReport.memory_available_mb ? `${preflightReport.memory_available_mb} MB` : 'N/A'}
                </span>
              </div>

              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px', background: 'rgba(255, 255, 255, 0.03)', borderRadius: '8px' }}>
                <span>CPU Cores</span>
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: '0.85rem' }}>
                  {preflightReport.cpu_cores || 'N/A'}
                </span>
              </div>
            </div>

            <button onClick={() => setPreflightReport(null)} className="btn btn-primary" style={{ width: '100%' }}>
              Close Report
            </button>
          </div>
        </div>
      )}

      {/* Add Server Modal */}
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
          <div className="glass-panel" style={{ width: '100%', maxWidth: '520px', padding: '32px' }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '24px' }}>
              <h2 style={{ fontSize: '1.35rem', fontWeight: 700 }}>Connect Remote Server</h2>
              <button
                onClick={() => setShowAddModal(false)}
                style={{ background: 'transparent', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', fontSize: '1.25rem' }}
              >
                ✕
              </button>
            </div>

            <form onSubmit={handleCreate} style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
              <div>
                <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>
                  Server Friendly Name
                </label>
                <input
                  required
                  type="text"
                  placeholder="e.g. Production US-East"
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                  style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff' }}
                />
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '3fr 1fr', gap: '12px' }}>
                <div>
                  <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Host / IP</label>
                  <input
                    required
                    type="text"
                    placeholder="192.168.1.100"
                    value={form.host}
                    onChange={(e) => setForm({ ...form, host: e.target.value })}
                    style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff' }}
                  />
                </div>
                <div>
                  <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Port</label>
                  <input
                    required
                    type="number"
                    value={form.port}
                    onChange={(e) => setForm({ ...form, port: Number(e.target.value) })}
                    style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff' }}
                  />
                </div>
              </div>

              <div>
                <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>Username</label>
                <input
                  required
                  type="text"
                  value={form.username}
                  onChange={(e) => setForm({ ...form, username: e.target.value })}
                  style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff' }}
                />
              </div>

              <div>
                <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>
                  SSH Host Fingerprint (optional SHA256 pin)
                </label>
                <input
                  type="text"
                  placeholder="SHA256:..."
                  value={form.known_host_fingerprint}
                  onChange={(e) => setForm({ ...form, known_host_fingerprint: e.target.value })}
                  style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff', fontFamily: 'var(--font-mono)', fontSize: '0.78rem' }}
                />
                <p style={{ marginTop: '6px', color: 'var(--text-muted)', fontSize: '0.75rem' }}>
                  When set, Moonships refuses SSH if the scanned host key does not match this fingerprint.
                </p>
              </div>

              <div>
                <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 500, marginBottom: '6px' }}>
                  SSH Private Key (Encrypted at rest with AES-256-GCM)
                </label>
                <textarea
                  rows={4}
                  placeholder="-----BEGIN OPENSSH PRIVATE KEY-----&#10;...&#10;-----END OPENSSH PRIVATE KEY-----"
                  value={form.private_key}
                  onChange={(e) => setForm({ ...form, private_key: e.target.value })}
                  style={{ width: '100%', padding: '10px 14px', borderRadius: '8px', background: 'rgba(0, 0, 0, 0.3)', border: '1px solid var(--border-color)', color: '#fff', fontFamily: 'var(--font-mono)', fontSize: '0.75rem' }}
                />
              </div>

              <div style={{ display: 'flex', gap: '12px', marginTop: '12px' }}>
                <button type="button" onClick={() => setShowAddModal(false)} className="btn btn-secondary" style={{ flex: 1 }}>
                  Cancel
                </button>
                <button type="submit" className="btn btn-primary" style={{ flex: 1 }}>
                  Save Server
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}
