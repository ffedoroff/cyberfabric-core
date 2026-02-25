
1. нужно ли сделать Tenant особым типом в RG и нужно ли для каждой RG иметь TenantID (ownership)?
        особый тип - не нужен
        для RG иметь tenant_id - уверен

2. rg membership должна ли содержать tenant_id? Если нет, то будут проблемы с повторящимися id в разных тенантах
        да добавил tenant_id - нужно еще подумать, может добавить resource_type + id

3. будет ли табличка tenants использоваться для проверки прав ownership или мы будем использовать только resource group?
        нужно чтобы rgs вызывал в authz access_evaluation_request - обратился в security context и спросил можно ли создать такой тенант/resource group

4. resource_kind в membership точно ли он нужен? 
        оставляем это поле, в том числе для избегания коллизий
        все клиенты membership обязаны гарантровать уникальные id среди всех тенантов в resource_kind

3. стоить ли делать подобные static_tr_plugin
        думаю да

4. в итоге будем иметь: (RG Rsolver sdk) И ((RG Plugin + RG Service) или (Vendor RG Plugin + Vendor RG Service))
        да, возможен сценарий работы authz без RG
        RG SDK на Read and Write
        RG Module (gateway)
        RG Plugin (полноценный сервис с базой данных, апи, сидинг)

5. определить human roles кто будет пользоваться RG:
        instance administrator - manage RG types, RG items, seeding (tenants)
        administrator - withing one tenant manage sub items (groups, departments, sub-tenants)
        apps - add/remove resource to group memberships, read memberships

6. в RG API нужно использовать переданный опциональный tenant_id, и если его нет, то SecurityContext.subject_tenant_id
        см ответ 3 : нужно чтобы rgs вызывал в authz access_evaluation_request


RG Management API (REST+GRPC+SDK) : read + write, it depends on Authz (access_evaluation_request). Client - any.
RG Access API (private REST+GRPC+SDK): read only, maybe internal api. Known client - authz (MTLS).

Authz depends on RG Access API sdk
RG Management API depends on Authz sdk

вместо RG Resolver делаем RG который будет R+W + Plugin + Reference RG Service






















tenant_hierarchy

  - Гарантировать tenant-scoping групп при возврате group_ids/root_group_id в constraints; PEP этому доверяет. docs/arch/authorization/DESIGN.md :1166 ,
    docs/arch/authorization/DESIGN.md:1171
  - Всегда добавлять tenant-предикат на ресурс как defense-in-depth рядом с group-предикатами. docs/arch/authorization/DESIGN.md:1173, docs/arch/
    authorization/AUTHZ_USAGE_SCENARIOS.md:1055



## Resource Group Properties

Resource groups are stored on the **vendor side** (in the vendor's Resource Group service). Cyber Fabric does not store the full group entity — only local projections for authorization (closure and membership tables).

The following properties are the **minimum** Cyber Fabric expects from the vendor's group model:

| Property | Type | Description |
|----------|------|-------------|
| `id` | UUID | Unique group identifier |
| `tenant_id` | UUID | Owning tenant (groups are tenant-scoped) |
| `parent_id` | UUID? | Parent group (NULL for root groups) |


роли:
instance administrator - manage RG types, RG items (tenants)
administrator - withing one tenant manage sub items (groups, departments, sub-tenants)
apps - add/remove resource to group memberships, read memberships

• Найденные проблемы (по убыванию критичности)

  1. Критично: противоречие в fail-closed логике по constraints: [].
     docs/arch/authorization/DESIGN.md:661 требует deny при пустом constraints, но docs/arch/authorization/AUTHZ_USAGE_SCENARIOS.md:846 и docs/arch/
     authorization/AUTHZ_USAGE_SCENARIOS.md:857 трактуют это как “no constraints / allow”. Это прямо меняет безопасность поведения PEP.
  2. Критично: противоречие по обязательности полей SecurityContext.
     docs/arch/authorization/DESIGN.md:474 и docs/arch/authorization/DESIGN.md:487 фиксируют subject_tenant_id и token_scopes как required, но docs/arch/
     authorization/AUTHN_JWT_OIDC_PLUGIN.md:285 и docs/arch/authorization/AUTHN_JWT_OIDC_PLUGIN.md:613 описывают subject_tenant_id как optional (и
     валидация только subject_id на docs/arch/authorization/AUTHN_JWT_OIDC_PLUGIN.md:631).
  3. Высокая: в RG PRD/DESIGN заявлена гарантия “reject cross-tenant membership links”, но описанный механизм её не доказывает.
     Требование есть в modules/system/resource-group/docs/PRD.md:118 и modules/system/resource-group/docs/PRD.md:240, но модель membership не содержит
     tenant ресурса (modules/system/resource-group/docs/DESIGN.md:679), а проверка сведена к caller tenant + group tenant (modules/system/resource-group/
     docs/DESIGN.md:730, modules/system/resource-group/docs/DESIGN.md:326). Это не равно проверке “tenant ресурса == tenant группы”.
  4. Средняя: несостыковка схем membership между auth-концептом и RG дизайном.
     Базовая auth-модель описывает membership как (resource_id, group_id) (docs/arch/authorization/RESOURCE_GROUP_MODEL.md:117, docs/arch/authorization/
     DESIGN.md:1329), а RG DESIGN вводит обязательный resource_kind (modules/system/resource-group/docs/DESIGN.md:682). Нужно явно синхронизировать SQL/
     Auth-модель декларирует “Depth is unlimited” (docs/arch/authorization/RESOURCE_GROUP_MODEL.md:77), а RG PRD/Design жёстко ограничивают max_depth <=
     10 (modules/system/resource-group/docs/PRD.md:302, modules/system/resource-group/docs/DESIGN.md:715).
  6. Средняя: документы auth расходятся с текущим SDK.
     resolver-sdk/src/models.rs:29).

  Краткий вывод

  В целом граница ответственности RG ↔ AuthZ выдержана (policy/SQL остаются вне RG), но в текущем виде есть несколько важных противоречий в auth-доках и
  заметные несостыковки RG PRD/DESIGN с общей моделью и текущими SDK-контрактами.
  Если хотите, следующим шагом могу дать точечный patch-план по документам (что и в каком файле править первым).

  • Найденные проблемы (по убыванию критичности)

  - HIGH: Конфликт по источнику истины для групп.
    Базовая модель говорит, что группы хранятся у вендора, а в Cyber Fabric только проекции: RESOURCE_GROUP_MODEL.md (/Users/roman/work/cyberfabric-core/
    docs/arch/authorization/RESOURCE_GROUP_MODEL.md:83).
    В resource-group PRD/DESIGN добавлен managed режим, где модуль сам хранит канонические сущности: PRD.md (/Users/roman/work/cyberfabric-core/modules/
    system/resource-group/docs/PRD.md:16), DESIGN.md (/Users/roman/work/cyberfabric-core/modules/system/resource-group/docs/DESIGN.md:25).
    Это архитектурный дрейф, если не зафиксирован как эволюция в общих auth docs/ADR.
  - HIGH: Конфликт по глубине иерархии.
    Общая auth-модель: глубина групп не ограничена: RESOURCE_GROUP_MODEL.md (/Users/roman/work/cyberfabric-core/docs/arch/authorization/
    RESOURCE_GROUP_MODEL.md:77).
    В RG PRD/DESIGN: max_depth ограничен и должен быть <=10: PRD.md (/Users/roman/work/cyberfabric-core/modules/system/resource-group/docs/PRD.md:299),
    PRD.md (/Users/roman/work/cyberfabric-core/modules/system/resource-group/docs/PRD.md:302), DESIGN.md (/Users/roman/work/cyberfabric-core/modules/
    system/resource-group/docs/DESIGN.md:735).
    Это меняет семантику модели и влияет на совместимость сценариев с глубокими деревьями.

поменяй логику во всех документах, напиши, что ширина и глубина может быть не ограничена, но рекомендуется иметь ограничения не более 10 в глубину по умолчанию (меняется в конфигах), ограничения нужны для быстрой работы, но могут быть выключены

  обнови документы modules/system/resource-group/docs и docs/arch/authorization

  - HIGH: Требование “запрещать cross-tenant membership” описано, но контракт записи не содержит данных для верификации tenant ресурса.
    Требование: PRD.md (/Users/roman/work/cyberfabric-core/modules/system/resource-group/docs/PRD.md:240).
    Но AddMembershipRequest несёт только group_id/resource_kind/resource_id: DESIGN.md (/Users/roman/work/cyberfabric-core/modules/system/resource-group/
    docs/DESIGN.md:281).
    Таблица membership тоже не хранит tenant ресурса: DESIGN.md (/Users/roman/work/cyberfabric-core/modules/system/resource-group/docs/DESIGN.md:697).
    В текущем виде детерминированная проверка cross-tenant связи требует внешнего источника/контракта, но он явно не описан.


проверь подробнее утверждение выше, кажется, что таблица Group содержит tenant_id, а метод AddMembershipRequest получает на вход ctx в котором содержится tenant_id, проверь, что я прав или нет


  - MEDIUM: Несоответствие схемы membership между общей auth-документацией и RG DESIGN.
    Общая модель/PEP SQL опирается на resource_group_membership(resource_id, group_id): RESOURCE_GROUP_MODEL.md (/Users/roman/work/cyberfabric-core/docs/arch/authorization/RESOURCE_GROUP_MODEL.md:188), DESIGN.md (/Users/roman/work/cyberfabric-core/docs/arch/authorization/DESIGN.md:1327).
    RG DESIGN вводит resource_kind и PK (group_id, resource_kind, resource_id): DESIGN.md (/Users/roman/work/cyberfabric-core/modules/system/resource-
    group/docs/DESIGN.md:702), DESIGN.md (/Users/roman/work/cyberfabric-core/modules/system/resource-group/docs/DESIGN.md:708).
    Нужно явно синхронизировать контракт (или зафиксировать, что это другой уровень хранения).
  - LOW: Поломанные ссылки в TOC AUTHZ_USAGE_SCENARIOS.md (неправильные backtick/anchor).
    authorization/TENANT_MODEL.md:259).


# tenant id
  1. Передавать tenant_id отдельным полем в запросе RG-read сейчас не обязательно только для default same-tenant пути.
     Потому что RG-read уже принимает ctx:

  - ResourceGroupReadClient (/Users/roman/work/cyberfabric-core/modules/system/resource-group/docs/PRD.md:440)

  2. Но для cross-tenant сценариев ctx сам по себе недостаточен.
     В SecurityContext есть только SecurityContext.subject_tenant_id, отдельного context tenant там нет:

  - SecurityContext (/Users/roman/work/cyberfabric-core/libs/modkit-security/src/context.rs:23)
  - subject_tenant_id (/Users/roman/work/cyberfabric-core/libs/modkit-security/src/context.rs:30)

  При этом AuthZ SDK явно поддерживает context_tenant_id (может отличаться от subject tenant):

  - AccessRequest::context_tenant_id  modules/system/authz-resolver/authz-resolver-sdk/src/pep/enforcer.rs:106
  - explicit_root_id_overrides_subject_tenant modules/system/authz-resolver/authz-resolver-sdk/src/pep/
    enforcer.rs:1015

  3. Возвращать tenant_id в RG API-схемах — полезно и по сути нужно.
     Почему: это единственный нормализованный способ для AuthZ plugin верифицировать, что строки реально из ожидаемого tenant (особенно при swappable provider, built-in/vendor).

  - provider swappable model (/Users/roman/work/cyberfabric-core/docs/arch/authorization/DESIGN.md:148)
  - tenant_id в RG read rows (/Users/roman/work/cyberfabric-core/modules/system/resource-group/docs/PRD.md:501)
  - membership tenant_id в RG read rows (/Users/roman/work/cyberfabric-core/modules/system/resource-group/docs/PRD.md:509)
  - caller-side PDP validation via tenant_id (/Users/roman/work/cyberfabric-core/modules/system/resource-group/docs/PRD.md:530)
