import React, { useState, useEffect } from 'react';
import { productName, colors } from '@bipremote/ui';
import './styles.css';

// Types
interface Session {
  id: string;
  host_fingerprint: string;
  client_identifier: string;
  state: 'pending' | 'approved' | 'active' | 'ended' | 'failed' | 'revoked';
  transport: 'direct_p2p' | 'relayed' | 'tcp_hole_punch';
  authorized_at?: string;
  approved_at?: string;
  started_at?: string;
  ended_at?: string;
  duration_seconds?: number;
  created_at: string;
}

interface SessionListResponse {
  sessions: Session[];
}

interface ApiResponse<T> {
  success?: boolean;
  error?: string;
  data?: T;
}

// Tauri invoke wrapper
async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

// Session row component
function SessionRow({ session, onApprove, onEnd }: { session: Session; onApprove: (id: string) => void; onEnd: (id: string) => void }) {
  const getStateColor = (state: string) => {
    switch (state) {
      case 'active': return colors.success;
      case 'pending': return '#fbbf24';
      case 'approved': return colors.accent;
      case 'ended': return '#94a3b8';
      case 'failed': return '#ef4444';
      case 'revoked': return '#ef4444';
      default: return '#94a3b8';
    }
  };

  const getStateLabel = (state: string) => {
    switch (state) {
      case 'active': return 'Ativa';
      case 'pending': return 'Pendente';
      case 'approved': return 'Aprovada';
      case 'ended': return 'Encerrada';
      case 'failed': return 'Falhou';
      case 'revoked': return 'Revogada';
      default: return state;
    }
  };

  const formatDuration = (seconds?: number) => {
    if (!seconds) return '-';
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    return `${h}h ${m}m ${s}s`;
  };

  const formatDate = (dateStr?: string) => {
    if (!dateStr) return '-';
    return new Date(dateStr).toLocaleString('pt-BR');
  };

  return (
    <tr className="session-row">
      <td className="session-id">{session.id.substring(0, 12)}...</td>
      <td className="host-fingerprint">{session.host_fingerprint.substring(0, 16)}...</td>
      <td className="client-id">{session.client_identifier}</td>
      <td>
        <span
          className="state-badge"
          style={{ backgroundColor: getStateColor(session.state) }}
        >
          {getStateLabel(session.state)}
        </span>
      </td>
      <td>{session.transport.replace('_', ' ')}</td>
      <td>{formatDate(session.created_at)}</td>
      <td>{formatDate(session.started_at)}</td>
      <td>{formatDate(session.ended_at)}</td>
      <td>{formatDuration(session.duration_seconds)}</td>
      <td className="actions">
        {session.state === 'pending' && (
          <button className="btn btn-primary btn-sm" onClick={() => onApprove(session.id)}>
            Aprovar
          </button>
        )}
        {session.state === 'active' && (
          <button className="btn btn-danger btn-sm" onClick={() => onEnd(session.id)}>
            Encerrar
          </button>
        )}
        {(session.state === 'approved' || session.state === 'pending') && (
          <button className="btn btn-danger btn-sm" onClick={() => onEnd(session.id)}>
            Cancelar
          </button>
        )}
      </td>
    </tr>
  );
}

// Main App component
export function App() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [filterState, setFilterState] = useState<string>('all');
  const [filterSearch, setFilterSearch] = useState('');

  // Load sessions
  const loadSessions = async () => {
    try {
      setLoading(true);
      const response = await invoke<SessionListResponse>('list_sessions');
      setSessions(response.sessions);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Erro ao carregar sessões');
    } finally {
      setLoading(false);
    }
  };

  // Approve session
  const handleApprove = async (sessionId: string) => {
    try {
      await invoke('approve_session', { session_id: sessionId });
      await loadSessions();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Erro ao aprovar sessão');
    }
  };

  // End session
  const handleEnd = async (sessionId: string) => {
    if (!confirm('Tem certeza que deseja encerrar esta sessão?')) return;
    try {
      await invoke('end_session', { session_id: sessionId });
      await loadSessions();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Erro ao encerrar sessão');
    }
  };

  // Load on mount and auto-refresh
  useEffect(() => {
    loadSessions();
  }, []);

  useEffect(() => {
    if (!autoRefresh) return;
    const interval = setInterval(loadSessions, 5000);
    return () => clearInterval(interval);
  }, [autoRefresh]);

  // Filter sessions
  const filteredSessions = sessions.filter(session => {
    if (filterState !== 'all' && session.state !== filterState) return false;
    if (filterSearch) {
      const search = filterSearch.toLowerCase();
      return (
        session.id.toLowerCase().includes(search) ||
        session.host_fingerprint.toLowerCase().includes(search) ||
        session.client_identifier.toLowerCase().includes(search)
      );
    }
    return true;
  });

  return (
    <div className="app">
      <header className="header">
        <div className="header-left">
          <div className="logo">
            <svg width="32" height="32" viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
              <rect width="32" height="32" rx="8" fill={colors.accent}/>
              <path d="M8 16L14 22L24 10" stroke="white" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
          </div>
          <div className="title-group">
            <h1>{productName} Admin</h1>
            <p className="subtitle">Console de Gerenciamento de Sessões Remotas</p>
          </div>
        </div>
        <div className="header-right">
          <div className="header-stats">
            <div className="stat">
              <span className="stat-value">{sessions.filter(s => s.state === 'active').length}</span>
              <span className="stat-label">Ativas</span>
            </div>
            <div className="stat">
              <span className="stat-value">{sessions.filter(s => s.state === 'pending').length}</span>
              <span className="stat-label">Pendentes</span>
            </div>
            <div className="stat">
              <span className="stat-value">{sessions.length}</span>
              <span className="stat-label">Total</span>
            </div>
          </div>
          <label className="toggle">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
            />
            <span>Auto-refresh (5s)</span>
          </label>
          <button className="btn btn-secondary" onClick={loadSessions} disabled={loading}>
            {loading ? 'Carregando...' : 'Atualizar'}
          </button>
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <span>{error}</span>
          <button onClick={() => setError(null)}>×</button>
        </div>
      )}

      <main className="main">
        <section className="filters">
          <div className="filter-group">
            <label>
              <span>Estado:</span>
              <select value={filterState} onChange={(e) => setFilterState(e.target.value)}>
                <option value="all">Todos</option>
                <option value="active">Ativa</option>
                <option value="pending">Pendente</option>
                <option value="approved">Aprovada</option>
                <option value="ended">Encerrada</option>
                <option value="failed">Falhou</option>
                <option value="revoked">Revogada</option>
              </select>
            </label>
            <label>
              <span>Buscar:</span>
              <input
                type="text"
                placeholder="ID, fingerprint, cliente..."
                value={filterSearch}
                onChange={(e) => setFilterSearch(e.target.value)}
              />
            </label>
          </div>
        </section>

        <section className="sessions-table-container">
          {loading ? (
            <div className="loading">Carregando sessões...</div>
          ) : filteredSessions.length === 0 ? (
            <div className="empty-state">
              <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
                <line x1="9" y1="9" x2="15" y2="15"/>
                <line x1="15" y1="9" x2="9" y2="15"/>
              </svg>
              <p>Nenhuma sessão encontrada</p>
              <p className="hint">Ajuste os filtros ou aguarde novas conexões</p>
            </div>
          ) : (
            <table className="sessions-table">
              <thead>
                <tr>
                  <th>ID da Sessão</th>
                  <th>Host (Fingerprint)</th>
                  <th>Cliente</th>
                  <th>Estado</th>
                  <th>Transporte</th>
                  <th>Criada em</th>
                  <th>Iniciada em</th>
                  <th>Encerrada em</th>
                  <th>Duração</th>
                  <th>Ações</th>
                </tr>
              </thead>
              <tbody>
                {filteredSessions.map(session => (
                  <SessionRow
                    key={session.id}
                    session={session}
                    onApprove={handleApprove}
                    onEnd={handleEnd}
                  />
                ))}
              </tbody>
            </table>
          )}
        </section>
      </main>

      <footer className="footer">
        <p>{productName} Admin Console - Versão 0.1.0</p>
      </footer>
    </div>
  );
}