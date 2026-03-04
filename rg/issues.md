# Resource Group — Issues & Contradictions

## 1. [MEDIUM] Зависимость RG Management API от AuthZ не задокументирована

В questions.md (line 34-38) зафиксировано:
> "RG Management API (REST+GRPC+SDK) : read + write, it depends on Authz (access_evaluation_request)"
> "Authz depends on RG Access API sdk"
> "RG Management API depends on Authz sdk"

В PRD и DESIGN описано только обратное направление — AuthZ plugin читает из RG через `ResourceGroupReadClient`. Но то, что RG Management API сам вызывает AuthZ `access_evaluation_request` для проверки прав на write-операции (создание групп, membership и т.д.) — нигде не задокументировано.

**Затронутые документы:**
- `modules/system/resource-group/docs/PRD.md` — Section 3 (Operational Concept), Section 10 (Dependencies)
- `modules/system/resource-group/docs/DESIGN.md` — Section 3.4 (Internal Dependencies), Section 3.5 (External Dependencies)

**Решение:** добавить AuthZ SDK как зависимость RG Management API для access control на write-операции. Описать циклическую зависимость: AuthZ → RG Read SDK, RG Write → AuthZ SDK.

---

## 2. [MEDIUM] UpdateGroupRequest в OpenAPI содержит group_type (required)

**openapi.yaml** (line 1076-1092):
```yaml
UpdateGroupRequest:
  type: object
  required: [group_type, name]
  properties:
    group_type: ...
    name: ...
    parent_id: ...
    external_id: ...
```

**PRD.md** (line 209):
> "update mutable fields (`name`, `external_id`)"

`group_type` не входит в мутабельные поля по PRD. Если group_type нельзя менять — он не должен быть required в PUT body. Если это idempotent PUT (full replacement), это нужно явно задокументировать + описать поведение при попытке смены group_type.

**Также:** `parent_id` в UpdateGroupRequest покрывает move_entity. Это корректно для REST (PUT совмещает update + move), но нужно описать валидацию: если parent_id изменился — это subtree move с cycle/type/depth checks.

**Решение:** либо убрать `group_type` из required в UpdateGroupRequest, либо задокументировать в PRD/DESIGN что PUT принимает group_type для валидации но не меняет его (и сервер возвращает ошибку при попытке изменения).

---

## 3. ~~[LOW] Closure таблица без PK/UNIQUE constraint~~ [RESOLVED]

**Решение:** добавлен `PRIMARY KEY (ancestor_id, descendant_id)` в `migration.sql` и `DESIGN.md`.
