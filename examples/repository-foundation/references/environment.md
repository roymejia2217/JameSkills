# Requisitos del entorno

Detectar OS/arch/stack/Git/host desde facts verificados. Git/tool ausentes: guiar fuente oficial y re-probe; Windows Pi puede requerir GitBash. Permiso host ausente: solicitar owner/admin, status Blocked/Unknown hasta API evidencia. Entorno offline: biblioteca local utilizable, gates remotos pendientes.

Los planes pueden añadir applies_when = {fact="os",equals="windows"} y el paso alternativo Linux; no tomar elección manual como Pass. Los URLs y comandos sugeridos usan IDs registrados por la aplicación, no endpoints arbitrarios de la suite.
