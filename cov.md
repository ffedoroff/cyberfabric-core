# Resource Group — разбивка по файлам > 500 строк

Сравнение: оригинал (6 608 loc) → новый модуль (20 368 loc), рост **3.1x**

| Строк | Тип | Файл | Функционал | Cypilot маркер |
|------:|:---:|------|-----------|----------------|
| 1990 | test | tests/group_service_test.rs | Тесты иерархии групп (TC-GRP-01..38) | `cpt-cf-resource-group-dod-testing-entity-hierarchy` |
| 1667 | test | tests/api_rest_test.rs | REST API тесты (HTTP, статусы) | `cpt-cf-resource-group-dod-testing-rest-api` |
| 1466 | docs | docs/features/0006-unit-testing.md | Спека плана тестирования | — |
| 1321 | test | tests/type_service_test.rs | Тесты CRUD типов, metadata_schema | `cpt-cf-resource-group-dod-testing-type-mgmt` |
| 1116 | docs | docs/DESIGN.md | Технический дизайн модуля | — |
| 809 | code | src/infra/storage/group_repo.rs | Persistence групп, closure table | `cpt-cf-resource-group-dod-entity-hier-hierarchy-engine` |
| 761 | docs | docs/PRD.md | Продуктовые требования | — |
| 629 | code | src/domain/group_service.rs | CRUD групп, валидация, циклы | `cpt-cf-resource-group-dod-entity-hier-entity-service` |
| 625 | test | tests/domain_unit_test.rs | Unit-тесты domain-логики | `cpt-cf-resource-group-dod-testing-error-conversions` |
| 600 | docs | docs/ADR/ADR-001-gts-type-system.md | ADR: система типов GTS | — |
| 565 | test | tests/tenant_filtering_db_test.rs | Интеграция AuthZ→SecureORM→SQL фильтр | — |

## Итого по категориям (только файлы > 500 строк)

| Категория | Строк | Доля |
|-----------|------:|-----:|
| **Тесты** | 6 168 | 51% |
| **Документация** | 3 943 | 33% |
| **Код** | 1 438 | 12% |

## Причины роста 2.7x по коду

1. **Расширение closure table** — closure table была и в оригинале, но новый модуль добавил cycle detection, move/reparent с пересчётом поддерева, query profile enforcement (group_repo 809 + group_service 629 = 1438 строк только в двух файлах)
2. **Система типов GTS** — полный CRUD типов с metadata_schema, allowed_parents, allowed_memberships (type_repo 426 + type_service 180)
3. **OData фильтрация** — маппинг SDK OData фильтров на SeaORM условия (odata_mapper 172 + sdk/odata/* ~130)
4. **REST слой с авторизацией** — tenant scoping через AccessScope + SecureORM, отдельные handlers/routes/dto/auth (dto 305 + auth 233 + handlers ~330 + routes ~320)
5. **Membership как отдельный домен** — в оригинале references, в новом — полноценный membership_service (147) + membership_repo (164)
6. **SDK крейт** — отдельный crate с моделями (341), API (103), OData, ошибками — не было в оригинале
7. **Миграции на SeaORM** — программные миграции вместо SQL файлов (183)

## Rust production code (без тестов) — все файлы

Итого: **4 935 строк** в 43 файлах

| Строк | Файл | Функционал | Cypilot маркер |
|------:|------|-----------|----------------|
| 809 | src/infra/storage/group_repo.rs | Persistence групп, closure table rebuild | `dod-entity-hier-hierarchy-engine` |
| 629 | src/domain/group_service.rs | CRUD групп, валидация, cycle detection | `dod-entity-hier-entity-service` |
| 426 | src/infra/storage/type_repo.rs | Persistence типов GTS | — |
| 341 | resource-group-sdk/src/models.rs | SDK модели, DTO, сериализация | `dod-sdk-foundation-sdk-models` |
| 305 | src/api/rest/dto.rs | REST DTO, OData маппинг | `dod-testing-odata-dto` |
| 233 | src/api/rest/auth.rs | Dual auth (mTLS + JWT), tenant scope | `dod-integration-auth-dual-auth` |
| 186 | src/domain/rg_service.rs | Фасад: orchestration domain-сервисов | — |
| 183 | src/infra/storage/migrations/initial.rs | SeaORM миграция, 5 таблиц | `dod-sdk-foundation-persistence` |
| 180 | src/domain/type_service.rs | CRUD типов, metadata_schema | `dod-type-mgmt-service-crud` |
| 172 | src/infra/storage/odata_mapper.rs | OData filter → SeaORM Condition | — |
| 164 | src/infra/storage/membership_repo.rs | Persistence членств | — |
| 162 | src/domain/error.rs | Доменные ошибки, маппинг | — |
| 147 | src/domain/membership_service.rs | CRUD членств, валидация типов | `dod-membership-service` |
| 140 | src/api/rest/routes/groups.rs | Axum роуты групп | `dod-entity-hier-rest-handlers` |
| 134 | src/api/rest/handlers/groups.rs | REST хендлеры групп | `dod-entity-hier-rest-handlers` |
| 129 | src/domain/seeding.rs | Автозаполнение типов из конфига | `dod-type-mgmt-seeding` |
| 126 | src/module.rs | ModKit module scaffold | `dod-sdk-foundation-module-scaffold` |
| 104 | src/api/rest/handlers/types.rs | REST хендлеры типов | `dod-type-mgmt-rest-handlers` |
| 103 | resource-group-sdk/src/api.rs | SDK трейты, клиентский API | `dod-sdk-foundation-sdk-traits` |
| 94 | src/api/rest/routes/types.rs | Axum роуты типов | `dod-type-mgmt-rest-handlers` |
| 90 | src/api/rest/handlers/memberships.rs | REST хендлеры членств | `dod-membership-rest-handlers` |
| 81 | src/api/rest/routes/memberships.rs | Axum роуты членств | `dod-membership-rest-handlers` |
| 81 | src/api/rest/error.rs | REST ошибки → HTTP статусы | — |
| 78 | resource-group-sdk/src/error.rs | SDK ошибки | `dod-sdk-foundation-sdk-errors` |
| 54 | src/domain/read_service.rs | Read-only запросы с авторизацией | `dod-integration-auth-read-service` |
| 51 | resource-group-sdk/src/odata/groups.rs | OData фильтр для групп | — |
| 45 | src/domain/validation.rs | Валидация полей (slug, path) | — |
| 38 | resource-group-sdk/src/odata/memberships.rs | OData фильтр для членств | — |
| 35 | resource-group-sdk/src/odata/hierarchy.rs | OData фильтр для иерархии | — |
| 27 | src/api/rest/routes/mod.rs | Роутинг: merge всех роутов | — |
| 24 | src/infra/storage/entity/resource_group.rs | SeaORM entity: resource_group | — |
| 23 | src/api/rest/handlers/mod.rs | Реэкспорт хендлеров | — |
| 20 | src/infra/storage/entity/gts_type.rs | SeaORM entity: gts_type | — |
| 19 | src/infra/storage/entity/resource_group_membership.rs | SeaORM entity: membership | — |
| 16 | src/infra/storage/entity/resource_group_closure.rs | SeaORM entity: closure | — |
| 14 | src/lib.rs | Crate root, реэкспорт | — |
| 14 | src/infra/storage/entity/gts_type_allowed_parent.rs | SeaORM entity: allowed_parent | — |
| 14 | src/infra/storage/entity/gts_type_allowed_membership.rs | SeaORM entity: allowed_membership | — |
| 14 | resource-group-sdk/src/lib.rs | SDK crate root | — |
| 10 | src/domain/mod.rs | Реэкспорт domain-слоя | — |
| 8 | src/infra/storage/migrations/mod.rs | Список миграций | — |
| 8 | resource-group-sdk/src/odata/mod.rs | OData модуль | `dod-sdk-foundation-rest-odata` |
| 7 | resource-group-sdk/src/odata/types.rs | OData фильтр для типов | — |
| 6 | src/infra/storage/mod.rs | Реэкспорт infra | — |
| 6 | src/infra/storage/entity/mod.rs | Реэкспорт entities | `dod-sdk-foundation-persistence` |
| 5 | src/api/rest/mod.rs | Реэкспорт REST | — |
| 1 | src/infra/mod.rs | Реэкспорт | — |
| 1 | src/api/mod.rs | Реэкспорт | — |
