# Protocolo remoto

Este diretório reserva os contratos para a comunicação entre Host, serviço privilegiado e backend.

## Regras iniciais

- O Host autentica com identidade criptográfica individual, não com um código compartilhado.
- O serviço privilegiado não expõe uma porta de escuta pública.
- A UI conversa com o serviço por IPC autenticado e autorizado.
- O plano de controle e o canal de mídia permanecem separados.
- Criptografia ponta a ponta, rotação e revogação devem ser especificadas antes da implementação.

Nenhuma captura de tela, entrada remota, execução de comando ou transferência de arquivo é implementada nesta etapa.
