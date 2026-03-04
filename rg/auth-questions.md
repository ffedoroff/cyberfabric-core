# AuthZ + RG Integration — Questions

## 1. RG Access API — отдельный crate или trait в существующем SDK?

Сейчас в DESIGN есть `ResourceGroupReadClient` (resolve_descendants, resolve_ancestors, resolve_memberships). RG Access API — это он и есть, или нужен отдельный trait/crate с другим контрактом (например, без `SecurityContext`, раз клиент — только AuthZ через MTLS)?

> RG Access API - "private REST+GRPC+SDK" и "MTLS" имеет доступ ко всем данным RG в рамках одного запрашиваемого тенанта, 
> самый частый и возможно, единственный запрос к RG Access API - верни мне доступню иерархию тенантов по tenant_id
> по факту это очень похоже  `ResourceGroupReadClient` (resolve_descendants, resolve_ancestors) без resolve_memberships
> предлагаю объявить новый ResourceGroupReadHierarchy в котором 

## 2. RG Access API — transport?

Написано "private REST+GRPC+SDK" и "MTLS". Сейчас authz-resolver работает in-process через ClientHub. RG Access API будет:
- a) in-process trait в ClientHub (как сейчас) + опционально GRPC для out-of-process deployment?
- b) всегда через GRPC/REST с MTLS (отдельный сервис)?

## 3. Циклическая зависимость при init — порядок загрузки?

AuthZ зависит от RG Access API SDK, RG Management API зависит от AuthZ SDK. При старте:
- RG Module инициализируется первым (регистрирует `ResourceGroupReadClient`)?
- Потом AuthZ (находит RG Read, регистрирует `AuthZResolverClient`)?
- Потом RG Management API начинает принимать запросы (находит AuthZ)?

Или RG Management и RG Access — это один модуль с двумя фазами init?

Ответ:

    Example hierarchy:
    ```text
      tenant T1 (11111111-1111-1111-1111-111111111111)
      └── tenant T7 (77777777-7777-7777-7777-777777777777)
      tenant T9 (99999999-9999-9999-9999-999999999999)
    ```

Пользователь тенанта T1 заходит на страницу со списком курсов. Ему доступны курсы своего T1 и дочернего T7 тенантов.
Как это работает:
JWT token содержит user_id, tenant_id аналоги (подробнее уточни в modules/system/authn-resolver)
пользователь отправляет запрос /api/lms/v1/courses с JWT токеном в котором есть tenant_id T1 (11111111-1111-1111-1111-111111111111)
courses rust сервис использует modules/system/authz-resolver/authz-resolver-sdk для дальнейшего получения access_evaluation_request
authz-resolver-sdk получает запрос от courses и отправляет приватный запрос в ResourceGroupReadHierarchy с tenant_id T1, user_id 123
authz-resolver-sdk получив список тенантов T1 и T7 и других constraints от ResourceGroupReadHierarchy преобразует их в constraints и отправляет в courses
courses получив constraints преобразует их в sql предикаты (фильтры) и добавляет их к своему запросу по выводу курсов
таким образом, authz ничего не знает про курсы, а courses ничего не знает про доступню юзеру иерархию тенантов или групп или еще чего-то
но в итоге, пользователь видит только то, что ему доступно по данной иерархии

## 4. AuthZ на read-операции RG — нужен?

RG Management API зависит от AuthZ для write-операций. А read-операции (listGroups, getGroup, listGroupDepth)?
- a) read тоже через AuthZ (access_evaluation_request → constraints → SecureORM)?
- b) read без AuthZ, только tenant scoping из SecurityContext?

## 5. RG Plugin — что именно он делает?

Написано "RG Plugin (полноценный сервис с базой данных, апи, сидинг)". В authz-resolver паттерн plugin = vendor-specific PDP. В RG контексте plugin — это:
- a) reference implementation бизнес-логики RG (как static-authz-plugin для AuthZ)?
- b) vendor-replaceable storage backend (вендор приносит свою БД/хранилище иерархий)?
- c) и то и другое?

## 6. "Возможен сценарий работы AuthZ без RG" — как именно?

Сейчас static-authz-plugin возвращает `owner_tenant_id` constraint без обращения к RG. Это и есть "AuthZ без RG"? Или имеется в виду что AuthZ plugin может использовать другой источник иерархий (не RG), и `Capability::GroupMembership` / `Capability::GroupHierarchy` просто не заявляются?

## 7. RG Management API — AuthZ granularity?

Какие action/resource передаются в `access_evaluation_request` при write-операциях RG?
- Один resource type на весь RG (`resource_group`) или раздельно (`resource_group_type`, `resource_group_entity`, `resource_group_membership`)?
- Actions: CRUD (`create`, `update`, `delete`) или domain-specific (`move_subtree`, `change_type`)?

## 8. Tenant context в RG Access API для AuthZ?

AuthZ plugin вызывает `resolve_descendants(ctx, root_id)`. Какой `SecurityContext` передаётся?
- a) service-level identity AuthZ-модуля (MTLS cert → system principal)?
- b) оригинальный SecurityContext конечного пользователя (passthrough)?

Это важно для tenant scoping: если AuthZ вызывает RG от имени system principal, то RG Access API не должен фильтровать по tenant.

## 9. Seed data — кто создаёт начальные типы и root groups?

RG Plugin описан как "полноценный сервис с сидингом". Seed включает:
- a) только resource_group_type (tenant, department, branch)?
- b) type + root tenant group (первый tenant)?
- c) конфигурируется per-deployment?

И связанный вопрос: seed выполняется до или после AuthZ init? (Если RG write зависит от AuthZ, а AuthZ ещё не ready при seed — deadlock.)

## 10. Vendor RG Plugin — контракт замены?

Написано "возможен Vendor RG Plugin + Vendor RG Service". Вендор заменяет:
- a) только storage (свой persistence adapter за тем же trait)?
- b) весь domain layer (своя валидация типов, closure logic)?
- c) полностью свой сервис, который просто реализует `ResourceGroupReadClient` + `ResourceGroupClient`?

И если вендор приносит свой RG — AuthZ plugin по-прежнему обращается к `ResourceGroupReadClient` через ClientHub, просто за ним стоит другая реализация?
