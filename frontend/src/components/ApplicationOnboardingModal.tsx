import React, { useEffect, useMemo, useState } from 'react';
import { AlertTriangle, CheckCircle, GitBranch, Search, Server, ShieldCheck } from 'lucide-react';
import { apiRequest } from '../api/client';

type Props = {
  projects: any[];
  servers: any[];
  onCancel: () => void;
  onComplete: (applicationId: number, deploymentId: number) => void;
};

type Inspection = {
  inspection_version: number;
  commit_sha: string;
  strategy: 'compose' | 'dockerfile' | 'static' | 'unresolved';
  confidence: 'high' | 'medium' | 'low';
  candidates: string[];
  dockerfile_path?: string | null;
  compose_file_path?: string | null;
  docker_context?: string | null;
  container_port_hint?: number | null;
  healthcheck_path_hint?: string | null;
  static_output_dir_hint?: string | null;
  package_manager?: string | null;
  framework_hint?: string | null;
  reasons: Array<{ code: string; path?: string | null; message: string }>;
};

type Plan = {
  plan_version: number;
  plan_fingerprint: string;
  revision_id: number;
  application_id: number;
  source: { repository: string; branch: string; commit_hash: string };
  target: { server_id: number; server_name: string };
  build_type: string;
  workload_type: string;
  compose_file_path?: string | null;
  dockerfile_path: string;
  docker_context: string;
  image_reference: string;
  container_port: number;
  published_port?: number | null;
  healthcheck_path?: string | null;
  healthcheck_port?: number | null;
  environment: Array<{ key: string; is_secret: boolean; value_fingerprint: string }>;
  domains: Array<{ hostname: string; port: number; https_enabled: boolean }>;
};

const fieldStyle: React.CSSProperties = {
  width: '100%',
  padding: '10px 14px',
  borderRadius: '8px',
  background: 'rgba(0, 0, 0, 0.3)',
  border: '1px solid var(--border-color)',
  color: '#fff',
};

const labelStyle: React.CSSProperties = {
  display: 'block',
  fontSize: '0.82rem',
  fontWeight: 600,
  marginBottom: '6px',
  color: 'var(--text-secondary)',
};

export function ApplicationOnboardingModal({ projects, servers, onCancel, onComplete }: Props) {
  const [environments, setEnvironments] = useState<any[]>([]);
  const [inspection, setInspection] = useState<Inspection | null>(null);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [createdApplicationId, setCreatedApplicationId] = useState<number | null>(null);
  const [busy, setBusy] = useState<'inspect' | 'plan' | 'deploy' | null>(null);
  const [error, setError] = useState('');

  const [form, setForm] = useState({
    name: '',
    slug: '',
    project_id: Number(projects[0]?.id || 0),
    environment_id: 0,
    server_id: Number(servers[0]?.id || 0),
    git_repository: '',
    git_branch: 'main',
    build_type: 'dockerfile',
    workload_type: 'single',
    compose_file_path: '',
    dockerfile_path: 'Dockerfile',
    docker_context: '.',
    docker_image: '',
    container_port: 80,
    published_port: '',
    healthcheck_path: '/health',
  });

  const canInspect = Boolean(
    form.project_id && form.server_id && form.git_repository.trim() && form.git_branch.trim()
  );

  const sourceChanged = useMemo(() => {
    if (!inspection) return false;
    return false;
  }, [inspection]);

  async function loadEnvironments(projectId: number) {
    if (!projectId) {
      setEnvironments([]);
      return;
    }
    try {
      const list = await apiRequest<any[]>(`/api/projects/${projectId}/environments`);
      setEnvironments(list);
      setForm((current) => ({
        ...current,
        environment_id: Number(
          list.some((environment) => environment.id === current.environment_id)
            ? current.environment_id
            : list[0]?.id || 0
        ),
      }));
    } catch (err: any) {
      setEnvironments([]);
      setError(err.message || 'Failed to load environments');
    }
  }

  useEffect(() => {
    if (!form.project_id && projects.length > 0) {
      setForm((current) => ({ ...current, project_id: Number(projects[0].id) }));
      return;
    }
    void loadEnvironments(form.project_id);
  }, [form.project_id]);

  function sourceFieldChanged(patch: Partial<typeof form>) {
    setInspection(null);
    setPlan(null);
    setCreatedApplicationId(null);
    setError('');
    setForm((current) => ({ ...current, ...patch }));
  }

  async function inspectSource() {
    if (!canInspect) return;
    setBusy('inspect');
    setError('');
    setPlan(null);
    try {
      const result = await apiRequest<Inspection>('/api/source-inspection', {
        method: 'POST',
        body: JSON.stringify({
          project_id: Number(form.project_id),
          server_id: Number(form.server_id),
          git_repository: form.git_repository.trim(),
          git_branch: form.git_branch.trim(),
        }),
      });
      setInspection(result);

      setForm((current) => {
        const next = { ...current };
        if (result.strategy === 'compose') {
          next.workload_type = 'compose';
          next.compose_file_path = result.compose_file_path || 'compose.yaml';
        } else if (result.strategy === 'dockerfile') {
          next.workload_type = 'single';
          next.build_type = 'dockerfile';
          next.dockerfile_path = result.dockerfile_path || 'Dockerfile';
          next.docker_context = result.docker_context || '.';
        }
        if (result.container_port_hint) next.container_port = result.container_port_hint;
        if (result.healthcheck_path_hint) next.healthcheck_path = result.healthcheck_path_hint;
        return next;
      });
    } catch (err: any) {
      setError(err.message || 'Source inspection failed');
    } finally {
      setBusy(null);
    }
  }

  async function createAndPlan(event: React.FormEvent) {
    event.preventDefault();
    if (!inspection) {
      setError('Inspect the selected source before preparing a deployment plan.');
      return;
    }
    if (!form.environment_id) {
      setError('The selected project needs an environment before application onboarding.');
      return;
    }

    setBusy('plan');
    setError('');
    try {
      const app = await apiRequest<any>('/api/applications', {
        method: 'POST',
        body: JSON.stringify({
          name: form.name.trim(),
          slug: form.slug.trim() || undefined,
          project_id: Number(form.project_id),
          environment_id: Number(form.environment_id),
          server_id: Number(form.server_id),
          git_repository: form.git_repository.trim(),
          git_branch: form.git_branch.trim(),
          build_type: form.build_type,
          workload_type: form.workload_type,
          compose_file_path:
            form.workload_type === 'compose' ? form.compose_file_path.trim() : undefined,
          dockerfile_path: form.dockerfile_path.trim(),
          docker_context: form.docker_context.trim(),
          docker_image:
            form.build_type === 'prebuilt_image' ? form.docker_image.trim() : undefined,
          container_port: Number(form.container_port),
          published_port: form.published_port ? Number(form.published_port) : undefined,
          healthcheck_path: form.healthcheck_path.trim() || undefined,
        }),
      });

      const prepared = await apiRequest<Plan>('/api/deployment-plans', {
        method: 'POST',
        body: JSON.stringify({
          application_id: app.id,
          commit_hash: inspection.commit_sha,
          commit_message: 'Prepared from v1.1 onboarding',
        }),
      });

      setCreatedApplicationId(app.id);
      setPlan(prepared);
    } catch (err: any) {
      setError(err.message || 'Failed to create application or prepare plan');
    } finally {
      setBusy(null);
    }
  }

  async function deployPlan() {
    if (!plan) return;
    setBusy('deploy');
    setError('');
    try {
      const deployment = await apiRequest<any>(
        `/api/deployment-plans/${plan.revision_id}/deploy`,
        { method: 'POST' }
      );
      onComplete(plan.application_id, deployment.deployment_id);
    } catch (err: any) {
      setError(err.message || 'Failed to queue planned deployment');
    } finally {
      setBusy(null);
    }
  }

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(0, 0, 0, 0.78)',
        backdropFilter: 'blur(8px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 50,
        padding: '20px',
      }}
    >
      <div
        className="glass-panel"
        style={{ width: '100%', maxWidth: '760px', padding: '28px', maxHeight: '92vh', overflowY: 'auto' }}
      >
        <div style={{ marginBottom: '20px' }}>
          <div style={{ fontSize: '0.72rem', color: 'var(--cyan-primary)', fontWeight: 700, letterSpacing: '0.08em' }}>
            SOURCE → INSPECT → PLAN → DEPLOY
          </div>
          <h2 style={{ fontSize: '1.4rem', fontWeight: 700, marginTop: '4px' }}>
            Deploy New Application
          </h2>
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginTop: '6px' }}>
            Moonships shows detected values and the immutable deployment plan before anything is queued.
          </p>
        </div>

        {error && (
          <div
            style={{
              display: 'flex',
              gap: '8px',
              padding: '10px 12px',
              marginBottom: '16px',
              borderRadius: '8px',
              background: 'rgba(239, 68, 68, 0.12)',
              color: '#fca5a5',
              fontSize: '0.82rem',
            }}
          >
            <AlertTriangle size={16} />
            <span>{error}</span>
          </div>
        )}

        {!plan ? (
          <form onSubmit={createAndPlan} style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
              <div>
                <label style={labelStyle}>Project</label>
                <select
                  value={form.project_id}
                  onChange={(event) =>
                    sourceFieldChanged({ project_id: Number(event.target.value), environment_id: 0 })
                  }
                  style={{ ...fieldStyle, background: '#111827' }}
                >
                  {projects.map((project) => (
                    <option key={project.id} value={project.id}>{project.name}</option>
                  ))}
                </select>
              </div>
              <div>
                <label style={labelStyle}>Environment</label>
                <select
                  value={form.environment_id}
                  onChange={(event) =>
                    setForm((current) => ({ ...current, environment_id: Number(event.target.value) }))
                  }
                  style={{ ...fieldStyle, background: '#111827' }}
                >
                  {environments.length === 0 && <option value={0}>No environment</option>}
                  {environments.map((environment) => (
                    <option key={environment.id} value={environment.id}>{environment.name}</option>
                  ))}
                </select>
              </div>
            </div>

            <div>
              <label style={labelStyle}>Application Name</label>
              <input
                required
                value={form.name}
                onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))}
                placeholder="e.g. API Gateway"
                style={fieldStyle}
              />
            </div>

            <div>
              <label style={labelStyle}>Target Server</label>
              <select
                value={form.server_id}
                onChange={(event) => sourceFieldChanged({ server_id: Number(event.target.value) })}
                style={{ ...fieldStyle, background: '#111827' }}
              >
                {servers.map((server) => (
                  <option key={server.id} value={server.id}>
                    {server.name} ({server.host})
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label style={labelStyle}>Git Repository</label>
              <input
                required
                value={form.git_repository}
                onChange={(event) => sourceFieldChanged({ git_repository: event.target.value })}
                placeholder="https://github.com/org/repo.git"
                style={{ ...fieldStyle, fontFamily: 'var(--font-mono)' }}
              />
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr auto', gap: '12px' }}>
              <div>
                <label style={labelStyle}>Branch</label>
                <input
                  required
                  value={form.git_branch}
                  onChange={(event) => sourceFieldChanged({ git_branch: event.target.value })}
                  style={{ ...fieldStyle, fontFamily: 'var(--font-mono)' }}
                />
              </div>
              <div style={{ alignSelf: 'end' }}>
                <button
                  type="button"
                  disabled={!canInspect || busy !== null}
                  onClick={inspectSource}
                  className="btn btn-secondary"
                  style={{ height: '40px' }}
                >
                  <Search size={15} />
                  {busy === 'inspect' ? 'Inspecting…' : 'Inspect Source'}
                </button>
              </div>
            </div>

            {inspection && (
              <div
                style={{
                  padding: '14px',
                  borderRadius: '9px',
                  background: 'rgba(6, 182, 212, 0.07)',
                  border: '1px solid rgba(6, 182, 212, 0.2)',
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px' }}>
                  <div>
                    <strong style={{ textTransform: 'capitalize' }}>{inspection.strategy}</strong>
                    <span style={{ marginLeft: '8px', color: 'var(--text-secondary)', fontSize: '0.8rem' }}>
                      confidence: {inspection.confidence}
                    </span>
                  </div>
                  <code style={{ fontSize: '0.75rem', color: 'var(--cyan-primary)' }}>
                    {inspection.commit_sha.slice(0, 12)}
                  </code>
                </div>
                <div style={{ marginTop: '8px', display: 'flex', flexDirection: 'column', gap: '4px' }}>
                  {inspection.reasons.map((reason) => (
                    <div key={reason.code} style={{ fontSize: '0.77rem', color: 'var(--text-secondary)' }}>
                      <CheckCircle size={12} style={{ verticalAlign: '-2px', marginRight: '6px' }} />
                      {reason.message}
                    </div>
                  ))}
                </div>
                {inspection.strategy === 'static' && (
                  <div style={{ marginTop: '8px', fontSize: '0.77rem', color: '#fbbf24' }}>
                    Static source detected. v1.1 does not invent a runtime automatically; keep an explicit Dockerfile override.
                  </div>
                )}
                {inspection.strategy === 'unresolved' && (
                  <div style={{ marginTop: '8px', fontSize: '0.77rem', color: '#fbbf24' }}>
                    Detection is intentionally unresolved. Choose the workload/build values explicitly below.
                  </div>
                )}
              </div>
            )}

            <div style={{ borderTop: '1px solid var(--border-color)', paddingTop: '16px' }}>
              <div style={{ fontSize: '0.8rem', fontWeight: 700, marginBottom: '12px' }}>
                Detected configuration — review or override
              </div>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px' }}>
                <div>
                  <label style={labelStyle}>Workload</label>
                  <select
                    value={form.workload_type}
                    onChange={(event) =>
                      setForm((current) => ({ ...current, workload_type: event.target.value }))
                    }
                    style={{ ...fieldStyle, background: '#111827' }}
                  >
                    <option value="single">Single container</option>
                    <option value="compose">Docker Compose</option>
                  </select>
                </div>
                <div>
                  <label style={labelStyle}>Build Type</label>
                  <select
                    value={form.build_type}
                    disabled={form.workload_type === 'compose'}
                    onChange={(event) =>
                      setForm((current) => ({ ...current, build_type: event.target.value }))
                    }
                    style={{ ...fieldStyle, background: '#111827' }}
                  >
                    <option value="dockerfile">Dockerfile</option>
                    <option value="prebuilt_image">Prebuilt image</option>
                  </select>
                </div>
              </div>

              {form.workload_type === 'compose' ? (
                <div style={{ marginTop: '12px' }}>
                  <label style={labelStyle}>Compose File</label>
                  <input
                    value={form.compose_file_path}
                    onChange={(event) =>
                      setForm((current) => ({ ...current, compose_file_path: event.target.value }))
                    }
                    placeholder="compose.yaml"
                    style={{ ...fieldStyle, fontFamily: 'var(--font-mono)' }}
                  />
                </div>
              ) : form.build_type === 'prebuilt_image' ? (
                <div style={{ marginTop: '12px' }}>
                  <label style={labelStyle}>Image</label>
                  <input
                    value={form.docker_image}
                    onChange={(event) =>
                      setForm((current) => ({ ...current, docker_image: event.target.value }))
                    }
                    placeholder="ghcr.io/org/app:tag"
                    style={{ ...fieldStyle, fontFamily: 'var(--font-mono)' }}
                  />
                </div>
              ) : (
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px', marginTop: '12px' }}>
                  <div>
                    <label style={labelStyle}>Dockerfile</label>
                    <input
                      value={form.dockerfile_path}
                      onChange={(event) =>
                        setForm((current) => ({ ...current, dockerfile_path: event.target.value }))
                      }
                      style={{ ...fieldStyle, fontFamily: 'var(--font-mono)' }}
                    />
                  </div>
                  <div>
                    <label style={labelStyle}>Build Context</label>
                    <input
                      value={form.docker_context}
                      onChange={(event) =>
                        setForm((current) => ({ ...current, docker_context: event.target.value }))
                      }
                      style={{ ...fieldStyle, fontFamily: 'var(--font-mono)' }}
                    />
                  </div>
                </div>
              )}

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '12px', marginTop: '12px' }}>
                <div>
                  <label style={labelStyle}>Container Port</label>
                  <input
                    type="number"
                    value={form.container_port}
                    onChange={(event) =>
                      setForm((current) => ({ ...current, container_port: Number(event.target.value) }))
                    }
                    style={fieldStyle}
                  />
                </div>
                <div>
                  <label style={labelStyle}>Published Port</label>
                  <input
                    type="number"
                    value={form.published_port}
                    onChange={(event) =>
                      setForm((current) => ({ ...current, published_port: event.target.value }))
                    }
                    placeholder="optional"
                    style={fieldStyle}
                  />
                </div>
                <div>
                  <label style={labelStyle}>Health Path</label>
                  <input
                    value={form.healthcheck_path}
                    onChange={(event) =>
                      setForm((current) => ({ ...current, healthcheck_path: event.target.value }))
                    }
                    placeholder="/health"
                    style={fieldStyle}
                  />
                </div>
              </div>
            </div>

            <div style={{ display: 'flex', gap: '12px', marginTop: '8px' }}>
              <button type="button" onClick={onCancel} className="btn btn-secondary" style={{ flex: 1 }}>
                Cancel
              </button>
              <button
                type="submit"
                disabled={!inspection || busy !== null || sourceChanged}
                className="btn btn-primary"
                style={{ flex: 1 }}
              >
                <ShieldCheck size={15} />
                {busy === 'plan' ? 'Preparing…' : 'Create & Prepare Plan'}
              </button>
            </div>
          </form>
        ) : (
          <div>
            <div
              style={{
                padding: '16px',
                borderRadius: '10px',
                border: '1px solid rgba(52, 211, 153, 0.25)',
                background: 'rgba(52, 211, 153, 0.07)',
                marginBottom: '16px',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '10px' }}>
                <CheckCircle size={18} color="var(--success)" />
                <strong>Immutable plan ready</strong>
              </div>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px', fontSize: '0.8rem' }}>
                <div>
                  <span style={{ color: 'var(--text-muted)' }}>Commit</span>
                  <div><code>{plan.source.commit_hash.slice(0, 12)}</code></div>
                </div>
                <div>
                  <span style={{ color: 'var(--text-muted)' }}>Target</span>
                  <div><Server size={12} style={{ verticalAlign: '-2px', marginRight: '4px' }} />{plan.target.server_name}</div>
                </div>
                <div>
                  <span style={{ color: 'var(--text-muted)' }}>Workload</span>
                  <div>{plan.workload_type} / {plan.build_type}</div>
                </div>
                <div>
                  <span style={{ color: 'var(--text-muted)' }}>Artifact</span>
                  <div style={{ wordBreak: 'break-all' }}>{plan.image_reference}</div>
                </div>
              </div>
            </div>

            <div style={{ marginBottom: '16px' }}>
              <label style={labelStyle}>Plan Fingerprint</label>
              <code
                style={{
                  display: 'block',
                  padding: '10px',
                  borderRadius: '8px',
                  background: '#090d16',
                  color: 'var(--cyan-primary)',
                  fontSize: '0.75rem',
                  wordBreak: 'break-all',
                }}
              >
                {plan.plan_fingerprint}
              </code>
            </div>

            <div style={{ fontSize: '0.78rem', color: 'var(--text-secondary)', marginBottom: '18px' }}>
              <GitBranch size={13} style={{ verticalAlign: '-2px', marginRight: '5px' }} />
              Queueing now will deploy this exact revision. Later application edits do not rewrite this plan.
            </div>

            <div style={{ display: 'flex', gap: '12px' }}>
              <button type="button" onClick={onCancel} className="btn btn-secondary" style={{ flex: 1 }}>
                Save Without Deploying
              </button>
              <button
                type="button"
                onClick={deployPlan}
                disabled={busy !== null}
                className="btn btn-primary"
                style={{ flex: 1 }}
              >
                {busy === 'deploy' ? 'Queueing…' : 'Deploy Exact Plan'}
              </button>
            </div>

            {createdApplicationId && (
              <div style={{ marginTop: '10px', fontSize: '0.7rem', color: 'var(--text-muted)' }}>
                Application #{createdApplicationId} has already been created.
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
