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
