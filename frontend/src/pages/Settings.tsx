import React, { useEffect, useState } from 'react';
import { apiRequest } from '../api/client';
import {
  Settings as SettingsIcon,
  Database,
  Shield,
  Server,
  Layers,
  HardDrive,
  CheckCircle,
  AlertTriangle,
  FileText,
  Key,
} from 'lucide-react';

export function Settings() {
  const [health, setHealth] = useState<any>(null);
  const [backups, setBackups] = useState<any[]>([]);
  const [backupQueued, setBackupQueued] = useState(false);
  const [loading, setLoading] = useState(true);

  async function loadOperations() {
    try {
      const [systemHealth, recentBackups] = await Promise.all([
        apiRequest<any>('/api/health').catch(() => null),
        apiRequest<any[]>('/api/operations/backups?limit=5').catch(() => []),
      ]);
      setHealth(systemHealth);
      setBackups(recentBackups);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadOperations();
  }, []);

  async function queueBackup() {
    setBackupQueued(true);
    try {
      await apiRequest('/api/operations/backups', { method: 'POST' });
      window.setTimeout(loadOperations, 1500);
    } catch (error: any) {
      alert(`Unable to queue backup: ${error.message}`);
    } finally {
      setBackupQueued(false);
    }
  }

  return (
    <div style={{ maxWidth: '1000px', margin: '0 auto' }}>
      <div style={{ marginBottom: '32px' }}>
        <h1 style={{ fontSize: '2rem', fontWeight: 700, letterSpacing: '-0.02em', marginBottom: '8px' }}>
          Platform Settings & Architecture
        </h1>
        <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem' }}>
          Control plane configuration, SQLite persistence engine, and security parameters
        </p>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
        {/* Core System Status */}
        <div className="card">
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '16px' }}>
            <Database size={22} color="var(--cyan-primary)" />
            <h2 style={{ fontSize: '1.2rem', fontWeight: 600 }}>SQLite-First Architecture</h2>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '16px' }}>
            <div style={{ padding: '16px', background: 'rgba(255, 255, 255, 0.02)', borderRadius: '8px', border: '1px solid var(--border-color)' }}>
              <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Database Backend</div>
              <div style={{ fontSize: '1.1rem', fontWeight: 700, color: 'var(--cyan-primary)', marginTop: '4px' }}>
                SQLite 3 (WAL Mode)
              </div>
              <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)', marginTop: '4px' }}>
                Zero external dependencies
              </div>
            </div>

            <div style={{ padding: '16px', background: 'rgba(255, 255, 255, 0.02)', borderRadius: '8px', border: '1px solid var(--border-color)' }}>
              <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Deployment Queue</div>
              <div style={{ fontSize: '1.1rem', fontWeight: 700, color: 'var(--indigo-primary)', marginTop: '4px' }}>
                Loco BackgroundQueue
              </div>
              <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)', marginTop: '4px' }}>
                Durable SQLite storage
              </div>
            </div>

            <div style={{ padding: '16px', background: 'rgba(255, 255, 255, 0.02)', borderRadius: '8px', border: '1px solid var(--border-color)' }}>
              <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase' }}>Secret Encryption</div>
              <div style={{ fontSize: '1.1rem', fontWeight: 700, color: 'var(--success)', marginTop: '4px' }}>
                AES-256-GCM
              </div>
              <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)', marginTop: '4px' }}>
                At-rest protection active
              </div>
            </div>
          </div>

          <div style={{ marginTop: '20px', padding: '12px 16px', borderRadius: '8px', background: 'rgba(6, 182, 212, 0.06)', border: '1px solid rgba(6, 182, 212, 0.2)' }}>
            <p style={{ fontSize: '0.85rem', color: '#e2e8f0', margin: 0, lineHeight: 1.6 }}>
              <strong>Zero-Infrastructure Guarantee:</strong> Moonships operates entirely out of <code>data/moonships.sqlite</code>.
              No PostgreSQL or Redis instances are required for local development, CI testing, or single-node production deployment.
            </p>
          </div>
        </div>

        {/* Security & Secret Management */}
        <div className="card">
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '16px' }}>
            <Shield size={22} color="var(--success)" />
            <h2 style={{ fontSize: '1.2rem', fontWeight: 600 }}>Security Posture</h2>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', fontSize: '0.9rem', color: 'var(--text-secondary)' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <CheckCircle size={16} color="var(--success)" />
              <span>SSH private keys and sensitive environment variables encrypted with AES-256-GCM before database insertion.</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <CheckCircle size={16} color="var(--success)" />
              <span>Structured command invocation: No arbitrary shell string interpolation or command injection vulnerabilities.</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <CheckCircle size={16} color="var(--success)" />
              <span>Concurrency locking: Max 1 active deployment per application returning <code>409 Conflict</code>.</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
              <CheckCircle size={16} color="var(--success)" />
              <span>Masked secret values: Application secrets are scrubbed to <code>••••••••</code> in all REST API responses.</span>
            </div>
          </div>
        </div>

        {/* Backup & Recovery Runbook */}
        <div className="card">
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '16px' }}>
            <HardDrive size={22} color="var(--indigo-primary)" />
            <h2 style={{ fontSize: '1.2rem', fontWeight: 600 }}>Backup & Disaster Recovery</h2>
          </div>

          <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', marginBottom: '16px' }}>
            Production backups use SQLite online backup, integrity verification, SHA-256, optional AES-256-GCM export encryption, and bounded retention.
          </p>

          <div style={{ display: 'flex', gap: '10px', alignItems: 'center', marginBottom: '16px' }}>
            <button className="btn btn-primary" onClick={queueBackup} disabled={backupQueued}>
              <HardDrive size={16} />
              <span>{backupQueued ? 'Queueing...' : 'Queue Verified Backup'}</span>
            </button>
            <span style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>
              Production also schedules backups automatically.
            </span>
          </div>

          {backups.length > 0 && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginBottom: '16px' }}>
              {backups.map((backup) => (
                <div
                  key={backup.id}
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    gap: '12px',
                    padding: '10px 12px',
                    border: '1px solid var(--border-color)',
                    borderRadius: '8px',
                    fontSize: '0.8rem',
                  }}
                >
                  <span style={{ fontFamily: 'monospace', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                    {backup.backup_path}
                  </span>
                  <span style={{ color: backup.verified ? 'var(--success)' : 'var(--warning)', whiteSpace: 'nowrap' }}>
                    {backup.status}{backup.verified ? ' · verified' : ''}
                  </span>
                </div>
              ))}
            </div>
          )}

          <div style={{ background: '#090d16', padding: '16px', borderRadius: '8px', border: '1px solid rgba(255, 255, 255, 0.08)', fontFamily: 'monospace', fontSize: '0.85rem' }}>
            <div style={{ color: 'var(--cyan-primary)', marginBottom: '8px' }}># Create + verify a backup:</div>
            <div style={{ color: '#e2e8f0' }}>cargo loco task backup:run</div>
            <div style={{ color: 'var(--cyan-primary)', margin: '16px 0 8px 0' }}># Verify an exported backup:</div>
            <div style={{ color: '#e2e8f0' }}>cargo loco task backup:verify path:/backup/moonships.sqlite</div>
            <div style={{ color: 'var(--cyan-primary)', margin: '16px 0 8px 0' }}># Offline restore into a staging target:</div>
            <div style={{ color: '#e2e8f0' }}>cargo loco task backup:restore path:/backup/moonships.sqlite target:/restore/moonships.sqlite confirm:RESTORE</div>
          </div>

          <div style={{ marginTop: '16px', display: 'flex', alignItems: 'center', gap: '8px', color: 'var(--warning)', fontSize: '0.85rem' }}>
            <AlertTriangle size={16} />
            <span>Always back up your <code>ENCRYPTION_KEY</code> alongside the SQLite file. Without it, encrypted SSH keys cannot be decrypted.</span>
          </div>
        </div>
      </div>
    </div>
  );
}
