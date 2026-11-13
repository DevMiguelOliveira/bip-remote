param(
    [Parameter(Mandatory = \False)]
    [string] \ = \
)

\Continue = 'Stop'

if (-not \) {
    Write-Error 'BIPREMOTE_TEST_DATABASE_URL não configurado.'
    exit 1
}

# Validar que o destino é exclusivamente bipremote_validation
if (\ -notmatch 'dbname=bipremote_validation(?:;|$)' -and \ -notmatch '/bipremote_validation(?:;|\?|$)') {
    Write-Error "Destino de migração não autorizado: apenas 'bipremote_validation' é permitido."
    exit 1
}

\ = Get-Command psql -ErrorAction SilentlyContinue
if (-not \) {
    Write-Error 'psql não encontrado. A validação de runtime requer PostgreSQL local.'
    exit 1
}

Write-Host "Destino validado: bipremote_validation"

# Lista de migrações em ordem
\ = @(
    "infrastructure/database/migrations/0001_initial.up.sql",
    "infrastructure/database/migrations/0002_multi_tenant_integrity.up.sql"
)

foreach (\ in \) {
    Write-Host "Executando \..."
    \ = psql "\" -v ON_ERROR_STOP=1 -1 -f "\"
    if (\ -ne 0) {
        Write-Error "Falha na migracao: \ (codigo de saida: \)"
        exit 1
    }
}

Write-Host "Executando testes de isolamento..."
\ = psql "\" -v ON_ERROR_STOP=1 -1 -f "tests/database/tenant-isolation.sql"
if (\ -ne 0) {
    Write-Error "Falha nos testes de isolamento (codigo de saida: \)"
    exit 1
}

Write-Host "Validação concluída com sucesso."
