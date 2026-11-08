import { productName } from '@bipremote/ui';

export function App() {
  return (
    <main className="shell">
      <section className="card" aria-labelledby="title">
        <p className="eyebrow">Fundação técnica</p>
        <h1 id="title">{productName} Admin</h1>
        <p className="description">
          Console preparado para gerenciamento seguro de organizações, unidades,
          técnicos, hosts e sessões auditáveis.
        </p>
        <div className="status" role="status">
          <span className="status-dot" />
          Contratos iniciais carregados
        </div>
      </section>
    </main>
  );
}
