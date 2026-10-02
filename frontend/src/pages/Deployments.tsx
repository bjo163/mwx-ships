import React, { useEffect, useState } from 'react';
import { Link } from 'react-router';
import { apiRequest } from '../api/client';
import {
  Rocket,
  RefreshCw,
  GitCommit,
  Clock,
  ArrowRight,
  CheckCircle,
  XCircle,
  AlertCircle,
  Server,
  Layers,
} from 'lucide-react';

export function Deployments() {
  const [deployments, setDeployments] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<string>('all');

  async function loadDeployments() {
    try {
      const data = await apiRequest<any[]>('/api/deployments');
      setDeployments(Array.isArray(data) ? data : []);
    } catch (e) {
      console.error('Failed to load deployments:', e);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadDeployments();
    const interval = setInterval(loadDeployments, 5000);
    return () => clearInterval(interval);
  }, []);

  const filteredDeployments =
    filter === 'all'
      ? deployments
      : deployments.filter((d) => d.status.toLowerCase() === filter.toLowerCase());

  return (
    <div style={{ maxWidth: '1200px', margin: '0 auto' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '32px' }}>
        <div>
          <h1 style={{ fontSize: '2rem', fontWeight: 700, letterSpacing: '-0.02em', marginBottom: '8px' }}>
            Deployment Activity
          </h1>
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem' }}>
            Real-time deployment audit log executed by Loco SQLite persistent queue
          </p>
        </div>
        <button onClick={loadDeployments} className="btn btn-secondary">
          <RefreshCw size={16} /> Refresh
        </button>
      </div>

      {/* Filter Tabs */}
      <div style={{ display: 'flex', gap: '8px', marginBottom: '24px' }}>
        {['all', 'success', 'failed', 'building', 'queued'].map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            style={{
              padding: '6px 14px',
              borderRadius: '6px',
              fontSize: '0.85rem',
              fontWeight: 500,
              textTransform: 'capitalize',
              border: filter === f ? '1px solid var(--indigo-primary)' : '1px solid var(--border-color)',
              background: filter === f ? 'var(--accent-glow)' : 'rgba(255, 255, 255, 0.02)',
              color: filter === f ? '#fff' : 'var(--text-secondary)',
              cursor: 'pointer',
            }}
          >
            {f}
          </button>
        ))}
      </div>

      {/* Deployments List */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
        {filteredDeployments.map((d) => {
          const isSuccess = d.status === 'success';
          const isFailed = d.status === 'failed';
          const isOngoing = !isSuccess && !isFailed;

          return (
            <div
              key={d.id}
              className="card"
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: '16px 20px',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
                <div
                  style={{
                    width: '40px',
                    height: '40px',
                    borderRadius: '8px',
                    background: isSuccess
                      ? 'rgba(16, 185, 129, 0.1)'
                      : isFailed
                      ? 'rgba(239, 68, 68, 0.1)'
                      : 'rgba(99, 102, 241, 0.1)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                  }}
                >
                  {isSuccess ? (
                    <CheckCircle size={20} color="var(--success)" />
                  ) : isFailed ? (
                    <XCircle size={20} color="var(--danger)" />
                  ) : (
                    <Rocket size={20} color="var(--indigo-primary)" className={isOngoing ? 'spin' : ''} />
                  )}
                </div>

                <div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '4px' }}>
                    <span style={{ fontWeight: 700, fontSize: '1rem', color: '#fff' }}>
                      Deployment #{d.id}
                    </span>
                    <span className={`status-badge status-${d.status}`}>{d.status}</span>
                  </div>

                  <div style={{ display: 'flex', alignItems: 'center', gap: '16px', fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                    <span>App #{d.application_id}</span>
                    <span>Server #{d.server_id}</span>
                    {d.commit_hash && (
                      <span style={{ display: 'flex', alignItems: 'center', gap: '4px', fontFamily: 'monospace' }}>
                        <GitCommit size={14} />
                        {d.commit_hash.substring(0, 7)}
                      </span>
                    )}
                    {d.error_message && (
                      <span style={{ color: 'var(--danger)', display: 'flex', alignItems: 'center', gap: '4px' }}>
                        <AlertCircle size={14} /> {d.error_message}
                      </span>
                    )}
                  </div>
                </div>
              </div>

              <div style={{ display: 'flex', alignItems: 'center', gap: '24px' }}>
                <div style={{ textAlign: 'right', fontSize: '0.8rem', color: 'var(--text-muted)' }}>
                  <div>Queued: {new Date(d.queued_at || d.created_at).toLocaleString()}</div>
                  {d.finished_at && <div>Finished: {new Date(d.finished_at).toLocaleTimeString()}</div>}
                </div>

                <Link
                  to={`/applications/${d.application_id}`}
                  className="btn btn-secondary"
                  style={{ padding: '8px 12px' }}
                >
                  <span>View Details</span>
                  <ArrowRight size={14} />
                </Link>
              </div>
            </div>
          );
        })}

        {filteredDeployments.length === 0 && !loading && (
          <div className="card" style={{ textAlign: 'center', padding: '48px', color: 'var(--text-secondary)' }}>
            <Rocket size={32} color="var(--text-muted)" style={{ margin: '0 auto 12px auto' }} />
            <h3>No deployments found</h3>
            <p style={{ fontSize: '0.9rem', marginTop: '6px' }}>
              Deployments triggered from applications will stream events here in real time.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
