# Spec: cloud-sync, backup cifrado y restore

R08/R09/R10 y SECURITY. Arquitectura local-first, cliente Google Drive sin servidor JameSkills. Inspiración 2FAS: appDataFolder propio, cuenta elegida, recuperar con misma cuenta y secreto de recuperación. Protocolo propio, nunca copiar código GPL ni backup de autenticador.

## Credenciales y provisionamiento

Owner crea Google Cloud project, habilita Drive API, consent screen, OAuth client Desktop, añade test users durante testing. Public client_id configurable; no insertar secreto confidencial en binario. Si token endpoint/client Desktop requiere campo client_secret distribuido, tratarlo como identificador público no boundary y seguir docs; nunca publicar un refresh token.
Scope Drive único https://www.googleapis.com/auth/drive.appdata. No scope full Drive. v1 UI usa etiqueta local de conexión + vault UUID; no obtiene email ni afirma identidad email sin scopes extra. El navegador permite elegir cuenta; "Cuenta vinculada" no significa cuenta nombrada por app sin evidencia.
OAuth: browser externo HTTPS Google exacto; PKCE S256, state random32, callback bind127.0.0.1 puerto0, path aleatorio, timeout180s, max4KiB request. Listener se crea antes abrir browser; validar method/path/state/code, one-use, cerrar. No 0.0.0.0 ni webview.
Token exchange TLS+timeout; refresh token keyring por AccountBindingId random; access token RAM; request URL/headers/code/state no logs. Disconnect cancela trabajos, borra cache OAuth keyring; ofrece revocar como acción concreta separada y no borra backup.
401 refresh una vez; invalid_grant ->NeedsReauth; 403 permissions/quota diferente; 429+5xx backoff jitter retry<=3, respetar Retry-After bounded60s y cancel.
Sin keyring disponible: conexión persistente Blocked; biblioteca y export cifrado manual siguen; nunca tokens plaintext fallback.

## Vault y protocolo criptográfico v1

Passphrase de backup distinta de credencial Google, entrada oculta y confirmación al crear. Mostrar irrecuperabilidad y descargar backup cifrado local; usuario decide recordar el material de desbloqueo en keyring (opt-in). Sin passphrase no hay cloud write.
VaultId UUID, master key aleatoria32, salt16 por vault key slot; KDF Argon2id versión19: m65536KiB, t3, p1, output32. Cifrado/wrap XChaCha20Poly1305 key32 nonce24 tag16. RustCrypto libs mantenidas y auditadas en T001; se usa biblioteca, no implementar primitives.
Parámetros fijos v1, parser rechaza otros antes gastar CPU; versión futura requiere migrator. Comparación auth tag de AEAD de library. RNG CSPRNG OS cada nonce/key/salt; fail RNG aborta. No deriving nonce de fecha o hash content.
Wrap key = Argon2id(passphrase,salt). Envuelto master_key ciphertext48. Master encrypt payload; clave cero al lock/shutdown; passphrase transitoria zeroize. No compartir una referencia de llave dentro UiEvent.

Header BINARIO tamaño176, todos enteros big-endian:
| Offset | Bytes | Campo |
|---|---|---|
|0|8|magic ASCII JSKSBK01|
|8|2|version=1|
|10|1|cipher_id=1|
|11|1|kdf_id=1|
|12|4|memory_kib=65536|
|16|4|iterations=3|
|20|4|parallelism=1|
|24|16|salt|
|40|16|vault UUID raw|
|56|16|snapshot UUID raw|
|72|24|wrap nonce fresh|
|96|24|payload nonce fresh|
|120|48|wrapped master ciphertext+tag|
|168|8|payload ciphertext length including tag|
Después header: payload ciphertext exact length; ningún byte trailing permitido.
WrapAAD = bytes[0..120] concatenado bytes[168..176] (128 bytes). PayloadAAD = header completo176. Probar manipulación de cada header field+wrappedkey+length+payload. Vault/snapshot IDs del payload deben coincidir header después decrypt.
Pasos seal: empaquetar bounded payload -> conocer length+tag -> header sin wrappedkey -> wrap(master,AAD) -> header completo -> seal payload. open: read header fixed limits -> derive -> unwrap -> decrypt whole payload/authenticate -> safe decompress -> domain.validate_snapshot -> VerifiedSnapshot.
En fichero grande se usa archivo temporal privado, límite256MiB cifrado y payload descomprimido agregado256MiB; AEAD one-shot requiere RAM hasta límite, medir budget. Si one-shot consume demasiado en fixture100MiB, bloque técnico y diseñar streaming AEAD version2 antes elevar límites, no fingir streaming con chunks no autenticados. Plaintext nunca se entrega a restore antes autenticar todo.

Cambiar passphrase no revoca copias antiguas ya exportadas. v1 rotación requiere vault nuevo+master nueva+snapshot completo reencrypted; explicit reset/retire old vault, old exports siguen decryptable vieja contraseña. No contraseña recovery alternativa sin design.

## Payload

ZIP seguro, único nesting esperado:
~~~
snapshot.json
bundles/<bundle_hash>.jskill
~~~
snapshot.json schema1:
- vault_id, snapshot_id, parents_snapshot_ids sorted únicos, device_id opaque UUID (no hostname), library_generation, created_at display.
- revisions metadata completas: id, skill_id, parents, kind, bundle_hash?, semantic_version; cada hash recalculado.
- heads por UUID; tombstones parte DAG; blobs para TODAS revisiones retenidas con contenido.
- versions_supported y canonical-hash version; ninguna machine installation/draft/token/evidence/path absoluta.
Probar parent loops/mismo UUID distinto hash/missing blob/bundle hash mismatch antes commit.
Budget total para ZIP externo+inner .jskill: 256MiB descomprimidos, layer2 explícito max, count <=100.000 global, bundle limits individuales; 2.000 archivos por bundle. No autoextract nested archives arbitrarios.

## Drive

RemoteSnapshotPort:
- GET https://www.googleapis.com/drive/v3/files?spaces=appDataFolder&q=...&pageSize=100&fields=nextPageToken,files(id,name,size,appProperties)
- Query appProperties app=jameskills/schema=1/vault_id; URL encode y escape valores UUID/controlados; no query texto skill.
- POST https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id,name,size,appProperties con parents=["appDataFolder"], name="snapshot-<uuid>.jskills-backup", mimeType application/octet-stream.
- GET /drive/v3/files/<file-id>?alt=media; ID viene lista validada, nunca URL remote arbitrary.
- appProperties solo app/schema/vault_id/snapshot_id; ciphertext SHA256 puede local, no skills names/slugs/topics.
Endpoints hardcoded allowlist proveedor; redirects disabled o mismo host HTTPS validados. Files are immutable: no PATCH to singleton HEAD. File names no unique guarantee.
Pagination completa; duplicate snapshot UUID+same ciphertext hash = retry duplicate safe; misma UUID distinta hash = integrity conflict no sobrescribir.
Read timeout60s; max download256MiB enforce metadata y stream bytes; no confiar size de server. Cache ciphertext opcional privado.
POST timeout ambiguo: relistar snapshot_id; si existe hash descargar/verificar, asociar en DB; si no visible puede reintentar con mismo encrypted bytes/snapshot_id y dedup posterior. No re-seal con mismo snapshot ID distinto bytes durante retry.
No GC automática de Drive v1. Botón reset cuenta/vault destructivo requiere export válido y plan de file IDs, files.delete permanente (appData no trash), autoridad explícita. Quota ofrece export offline y clear plan, no silently delete old recovery.

## Merge causal y conflictos

Dos capas: snapshots DAG transporte; revisions DAG contenido. Cada snapshot completo puede transportar ancestros y heads actuales; parents snapshot trazan lo observado, nunca asumir un listado single-page tiene toda historia.
Merge reduce heads por relación ancestor: descendant elimina ancestor; inconexos conservados como conflicto. No last-write-wins por reloj/version.
Tombstone elimina heads que observó y sus ancestros; concurrent descendant/branch no observado se muestra delete-vs-edit conflict. No resurrección silenciosa.
Resolve local/remoto/merged crea content/tombstone NUEVO con parents todos los heads conflictivos; biblioteca head única; publish semantic version nueva. "Conservar ambos" clone content UUID nuevo y resolución causal original; no dos slugs con overwrite.
Dos dispositivos crean vault antes observar otro -> mostrar selector de vaults; nunca merge keys/vaults automáticamente.
Sync interval solo app activa min5min + debounce30s de cambios; manual SyncNow; una ejecución por vault+account. Inicio offline sigue pending sin bucle.
Snapshot captura generation G. Al upload confirmar G vs current generation; si cambió quedar Pending y siguientes pass. UI synced solo si heads locale/remote todos conocidos convergen y última operación completa.
Offline deletion/edit/update pruebas simuladas con reloj invertido y order aleatorio.

## Restore y recuperación

Preview cuenta/archivo -> decrypt+validate staging -> mostrar cantidades/conflictos/diffs -> usuario elegir merge o reemplazar lógica biblioteca. Reemplazar no destruye historia: crea baseline/import revisiones y tombstones como operación transaccional; export local actual cifrado primero. Restore nunca reinstala agentes ni configura paths/keyring/GitHub.
Journal RestorePrepared -> RecoveryWritten -> Validated -> Committed. Recuperación tras crash, DB+blobs coherentes, rollback antes commit; después commit no borrar revisión restaurada para parecer cancelada.
Wrong password/corruption = mensaje seguro igual, ninguna mutación, retry limitado UI.
Lost keyring: reconectar OAuth y passphrase restore funciona otro OS. Lost password: no recovery de ciphertext; conserva biblioteca local si existe, ofrece vault nuevo.
Tests crypto vectors/field tamper/RNG fail/size bounds, OAuth fake loopback, HTTP fake paginado/timeout/429401, DAG property tests y restore failpoints; integración Google real requiere cuenta owner y evidencia. Ninguna prueba normal usa Drive personal.

## Material de sesión necesario para sellar snapshots

UnlockedVault privado contiene vault_id, master_key32, wrapping_key32 derivada Argon2id, salt16 y parámetros fijos. Se zeroiza al lock/shutdown. La passphrase se destruye tras derivación; no se cachea contraseña. El wrapping_key se requiere porque cada nuevo header/AAD exige volver a envolver master con nonce nuevo. Cache opt-in keyring guarda material versionado master+wrapping_key+salt+vault_id, protegido por el backend OS; nunca solo master fingiendo que puede crear un wrap nuevo sin derivación. Al restaurar otro equipo, passphrase+header reconstruyen esas llaves; keyring no forma parte backup. Cada operación verifica vault_id/salt/key-slot contra envelope, no reutiliza keys de otra cuenta/vault.

CryptoProvider::create_vault(passphrase)->UnlockedVault genera master/salt; CryptoProvider::unlock_vault(envelope,passphrase)->UnlockedVault valida wrap/header; open_with_vault(envelope,vault)->VerifiedSnapshot autentica payload y valida. CryptoPort::open(password) puede componer unlock+open con llaves efímeras. SyncService::unlock usa los primeros métodos vía CryptoPort y nunca envía llaves en UiEvent.
