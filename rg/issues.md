# Проблемы документации Resource Group (RG)

> Обнаружены при сверке PRD.md и DESIGN.md с reference-файлами: `rg/migration.sql`, `rg/openapi.yaml`, `rg/size.md`.

## Medium

### 1. openapi.yaml: поле `depth` отсутствует в Group schema

- **Где**: `rg/openapi.yaml` — schema `Group` (строки 578–603)
- **Суть**: PRD требует `depth` в ответе списка групп (`cpt-cf-resource-group-fr-list-groups-depth`). DESIGN §3.3 описывает depth-based фильтрацию. В openapi.yaml `GroupPage` example содержит `depth: 0`, но сама schema `Group` не определяет поле `depth`.
- **Воздействие**: клиент, генерирующий типы по openapi, не получит поле `depth`.
- **Решение**: добавить `depth: { type: integer }` в schema `Group` и в `required`.

### 2. openapi.yaml: противоречие по `tenant_id` в Membership list

- **Где**: `rg/openapi.yaml` — `listMemberships` description (строка 326) vs schema `Membership` + `MembershipPage` example
- **Суть**: description говорит _"tenant_id is not returned in list responses"_, но schema `Membership` определяет `tenant_id` как required, и MembershipPage example содержит `tenant_id`.
- **Воздействие**: неоднозначность для потребителей API — возвращается `tenant_id` или нет.
- **Решение**: привести в соответствие — либо убрать `tenant_id` из Membership list response schema и примеров, либо убрать фразу "not returned" из description.

## Low

### 3. DESIGN: индекс `idx_rgm_resource_id` не задокументирован

- **Где**: `rg/migration.sql` строка 69 vs DESIGN §3.7 (membership indexes)
- **Суть**: migration.sql создаёт индекс `idx_rgm_resource_id` на `(resource_id)` отдельно от `(resource_type, resource_id)`. DESIGN §3.7 документирует только `(resource_type, resource_id)`.
- **Воздействие**: расхождение между фактической схемой и документацией.
- **Решение**: добавить `idx_rgm_resource_id` в DESIGN §3.7 или удалить из migration, если он избыточен (покрывается составным индексом).

### 4. openapi.yaml: Group response не включает `created`/`modified`

- **Где**: `rg/openapi.yaml` — schema `Group` vs PRD §5.2 (entity fields include timestamps)
- **Суть**: PRD определяет timestamps (`created`, `modified`) как поля entity. openapi.yaml не включает их в Group response schema.
- **Воздействие**: API не возвращает метаданные о времени создания/обновления группы.
- **Решение**: если это by design — зафиксировать решение явно. Если нет — добавить `created` и `modified` в Group schema.

### 5. openapi.yaml: PageInfo schema — несоответствие required-полей

- **Где**: `rg/openapi.yaml` — schema `PageInfo` (строки 506–514)
- **Суть**: schema определяет `required: [limit]`, а примеры и использование везде оперируют полями `top` и `skip`. Поле `limit` нигде больше не используется.
- **Воздействие**: сгенерированные клиенты получат неверную структуру PageInfo.
- **Решение**: заменить `required: [limit]` и свойство `limit` на `top` + `skip` в соответствии с примерами.
