# Modelo de segurança

## Identidades

Cada Host terá uma identidade criptográfica individual emitida e revogável. O fingerprint é identificador operacional, não segredo. Chaves privadas permanecem no host e não entram em logs ou no banco.

## Autorização

As decisões de acesso devem considerar organização, unidade, perfil do operador, estado do Host, estado da sessão e finalidade registrada. O backend é a autoridade do plano de controle; o Host valida a autorização recebida e a validade criptográfica do canal.

## Serviço privilegiado

O serviço privilegiado não deve ser usado como atalho para a UI. O IPC precisa autenticar o chamador, autorizar cada operação, limitar a superfície exposta e registrar ações relevantes.

## Sessões

Uma sessão possui pedido, aprovação, ativação, encerramento e revogação. Eventos de sessão são versionados e auditados. A criptografia ponta a ponta e a rotação de chaves devem ser especificadas antes dos recursos de mídia e controle.

## Atualizações

Atualizações do Host devem ser assinadas e verificadas antes da instalação. O mecanismo de rollback e a revogação de versões comprometidas são requisitos de produção.
