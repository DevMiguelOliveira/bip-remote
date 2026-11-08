$ErrorActionPreference = 'Stop'

$requiredPaths = @(
  'apps/admin',
  'agents/host',
  'services/server',
  'packages/ui',
  'protocols/api',
  'protocols/events',
  'protocols/remote',
  'infrastructure/database',
  'infrastructure/development',
  'infrastructure/deployment',
  'docs/requirements',
  'docs/architecture',
  'docs/security',
  'docs/decisions',
  'docs/operations',
  'tests/unit',
  'tests/integration',
  'tests/e2e',
  'tests/security',
  'tests/load',
  '.github/workflows'
)

$missing = $requiredPaths | Where-Object { -not (Test-Path $_) }
if ($missing) {
  Write-Error ("Missing required paths: " + ($missing -join ', '))
}

Write-Output ("Validated {0} required paths." -f $requiredPaths.Count)
