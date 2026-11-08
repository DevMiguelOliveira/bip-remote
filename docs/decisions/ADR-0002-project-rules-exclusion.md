# ADR-0002: Exclusão de `PROJECT_RULES.md` da fundação inicial

**Status:** Aceito  
**Data:** 2026-11-08

## Contexto

Antes desta etapa, `docs/PROJECT_RULES.md` estava ausente do working tree. O
índice continha uma entrada vazia para esse caminho. Também existiam versões
históricas mantidas por referências internas do ambiente Git.

O responsável pelo projeto decidiu manter o arquivo excluído da fundação
inicial. Essa decisão não autoriza restaurar, copiar ou incorporar qualquer
versão histórica ao working tree ou ao primeiro commit.

## Decisão

`docs/PROJECT_RULES.md` permanecerá excluído do primeiro commit raiz local do
BIPRemote.

As referências internas existentes não serão apagadas nem utilizadas como base
do commit. Nenhum conteúdo dessas referências será incorporado à fundação
inicial.

## Consequências

- O primeiro commit não conterá `docs/PROJECT_RULES.md`.
- O estado das referências internas será preservado.
- Uma eventual inclusão futura dependerá de uma decisão técnica explícita.
- A ausência do arquivo é intencional e não deve ser tratada como exclusão
  acidental durante a preparação do commit.
