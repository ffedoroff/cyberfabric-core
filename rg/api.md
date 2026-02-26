# Resource Group API (кратко)

База: `https://api.example.com`

Ниже для каждого URL показаны методы и один совмещенный пример `request/response`.
Все UUID и имена в примерах синхронизированы с `rg/seeding.sql`.

```text
tenant T1 (11111111-1111-1111-1111-111111111111)
├── department D2 (22222222-2222-2222-2222-222222222222)
│   ├── branch B3 (33333333-3333-3333-3333-333333333333)
│   │   └── resource R4
│   └── resource R5
├── resource R4
├── resource R6
└── tenant T7 (77777777-7777-7777-7777-777777777777)
    └── resource R8
tenant T9 (99999999-9999-9999-9999-999999999999)
└── resource R0
```

## 1) `/api/resource-group/v1/types`

`POST /api/resource-group/v1/types`
`GET /api/resource-group/v1/types`

```jsonc
{
  // Request (POST)
  "request": {
    "code": "department",
    "ancestors": ["tenant"]
  },
  // Response (POST)
  "response": {
    "code": "department",
    "ancestors": ["tenant"]
  }
}
```

## 2) `/api/resource-group/v1/types/{code}`

`GET /api/resource-group/v1/types/department`
`PUT /api/resource-group/v1/types/department`
`DELETE /api/resource-group/v1/types/department`

```jsonc
{
  // Request (GET)
  "request": null,
  // Response (GET)
  "response": {
    "code": "department",
    "ancestors": ["tenant"]
  }
}
```

## 3) `/api/resource-group/v1/groups`

`POST /api/resource-group/v1/groups`

```jsonc
{
  // Request (POST)
  "request": {
    "group_type": "department",
    "name": "D2",
    "parent_id": "11111111-1111-1111-1111-111111111111",
    "tenant_id": "11111111-1111-1111-1111-111111111111",
    "external_id": "D2"
  },
  // Response (POST)
  "response": {
    "id": "22222222-2222-2222-2222-222222222222",
    "parent_id": "11111111-1111-1111-1111-111111111111",
    "group_type": "department",
    "name": "D2",
    "tenant_id": "11111111-1111-1111-1111-111111111111",
    "external_id": "D2",
    "created": "2026-02-25T12:00:00Z",
    "modified": null
  }
}
```

## 4) `/api/resource-group/v1/groups/{id}`

`GET /api/resource-group/v1/groups/22222222-2222-2222-2222-222222222222`
`PUT /api/resource-group/v1/groups/22222222-2222-2222-2222-222222222222`
`DELETE /api/resource-group/v1/groups/22222222-2222-2222-2222-222222222222?force=true|false`

```jsonc
{
  // Request (GET)
  "request": null,
  // Response (GET)
  "response": {
    "id": "22222222-2222-2222-2222-222222222222",
    "parent_id": "11111111-1111-1111-1111-111111111111",
    "group_type": "department",
    "name": "D2",
    "tenant_id": "11111111-1111-1111-1111-111111111111",
    "external_id": "D2"
  }
}
```

## 4.1) `/api/resource-group/v1/groups-filter`

`POST /api/resource-group/v1/groups-filter`

```jsonc
{
  // Request (POST): аналог трех вызовов GET /groups/{id} одним запросом
  "request": {
    "ids": [
      "22222222-2222-2222-2222-222222222222",
      "22222222-2222-2222-2222-222222222221",
      "22222222-2222-2222-2222-222222222224"
    ],
    "tenant_id": "11111111-1111-1111-1111-111111111111"
  },
  // Response (POST)
  "response": {
    "items": [
      {
        "id": "22222222-2222-2222-2222-222222222222",
        "parent_id": "11111111-1111-1111-1111-111111111111",
        "group_type": "department",
        "name": "D2",
        "tenant_id": "11111111-1111-1111-1111-111111111111",
        "external_id": "D2"
      }
    ],
    "not_found_ids": [
      "22222222-2222-2222-2222-222222222221",
      "22222222-2222-2222-2222-222222222224"
    ]
  }
}
```

## 5) `/api/resource-group/v1/groups/{id}/ancestors`

`GET /api/resource-group/v1/groups/33333333-3333-3333-3333-333333333333/ancestors`

```jsonc
{
  // Request (GET)
  "request": null,
  // Response (GET)
  "response": {
    "group": {
      "id": "33333333-3333-3333-3333-333333333333",
      "parent_id": "22222222-2222-2222-2222-222222222222",
      "group_type": "branch",
      "name": "B3",
      "tenant_id": "11111111-1111-1111-1111-111111111111",
      "external_id": "B3"
    },
    "items": [
      {
        "group": {
          "id": "22222222-2222-2222-2222-222222222222",
          "group_type": "department",
          "name": "D2",
          "tenant_id": "11111111-1111-1111-1111-111111111111"
        },
        "depth": 1
      },
      {
        "group": {
          "id": "11111111-1111-1111-1111-111111111111",
          "group_type": "tenant",
          "name": "T1",
          "tenant_id": "11111111-1111-1111-1111-111111111111"
        },
        "depth": 2
      }
    ]
  }
}
```

## 6) `/api/resource-group/v1/groups/{id}/descendants`

`GET /api/resource-group/v1/groups/11111111-1111-1111-1111-111111111111/descendants`

```jsonc
{
  // Request (GET)
  "request": null,
  // Response (GET)
  "response": {
    "group": {
      "id": "11111111-1111-1111-1111-111111111111",
      "group_type": "tenant",
      "name": "T1",
      "tenant_id": "11111111-1111-1111-1111-111111111111",
      "external_id": "T1"
    },
    "items": [
      {
        "group": {
          "id": "22222222-2222-2222-2222-222222222222",
          "parent_id": "11111111-1111-1111-1111-111111111111",
          "group_type": "department",
          "name": "D2",
          "tenant_id": "11111111-1111-1111-1111-111111111111"
        },
        "depth": 1
      },
      {
        "group": {
          "id": "77777777-7777-7777-7777-777777777777",
          "parent_id": "11111111-1111-1111-1111-111111111111",
          "group_type": "tenant",
          "name": "T7",
          "tenant_id": "11111111-1111-1111-1111-111111111111"
        },
        "depth": 1
      },
      {
        "group": {
          "id": "33333333-3333-3333-3333-333333333333",
          "parent_id": "22222222-2222-2222-2222-222222222222",
          "group_type": "branch",
          "name": "B3",
          "tenant_id": "11111111-1111-1111-1111-111111111111"
        },
        "depth": 2
      }
    ]
  }
}
```

## 7) `/api/resource-group/v1/groups/{id}/references`

`POST /api/resource-group/v1/groups/33333333-3333-3333-3333-333333333333/references`
`DELETE /api/resource-group/v1/groups/33333333-3333-3333-3333-333333333333/references`

```jsonc
{
  // Request (POST)
  "request": {
    "resource_type": "resource",
    "resource_id": "R4",
    "tenant_id": "11111111-1111-1111-1111-111111111111"
  },
  // Response (POST)
  "response": {
    "group_id": "33333333-3333-3333-3333-333333333333",
    "resource_type": "resource",
    "resource_id": "R4",
    "tenant_id": "11111111-1111-1111-1111-111111111111"
  }
}
```

## 8) `/api/resource-group/v1/groups-descendants`

`POST /api/resource-group/v1/groups-descendants`

```jsonc
{
  // Request (POST)
  "request": {
    "ids": [
      "11111111-1111-1111-1111-111111111111",
      "22222222-2222-2222-2222-222222222222"
    ]
  },
  // Response (POST)
  "response": {
    "items": [
      {
        "group": {
          "id": "11111111-1111-1111-1111-111111111111",
          "group_type": "tenant",
          "name": "T1",
          "tenant_id": "11111111-1111-1111-1111-111111111111"
        },
        "descendants": [
          {
            "group": {
              "id": "22222222-2222-2222-2222-222222222222",
              "group_type": "department",
              "name": "D2",
              "tenant_id": "11111111-1111-1111-1111-111111111111"
            },
            "depth": 1
          },
          {
            "group": {
              "id": "33333333-3333-3333-3333-333333333333",
              "group_type": "branch",
              "name": "B3",
              "tenant_id": "11111111-1111-1111-1111-111111111111"
            },
            "depth": 2
          }
        ]
      }
    ]
  }
}
```

## 9) `/api/resource-group/v1/groups-ancestors`

`POST /api/resource-group/v1/groups-ancestors`

```jsonc
{
  // Request (POST)
  "request": {
    "ids": [
      "33333333-3333-3333-3333-333333333333",
      "77777777-7777-7777-7777-777777777777"
    ]
  },
  // Response (POST)
  "response": {
    "items": [
      {
        "group": {
          "id": "33333333-3333-3333-3333-333333333333",
          "group_type": "branch",
          "name": "B3",
          "tenant_id": "11111111-1111-1111-1111-111111111111"
        },
        "ancestors": [
          {
            "group": {
              "id": "22222222-2222-2222-2222-222222222222",
              "group_type": "department",
              "name": "D2",
              "tenant_id": "11111111-1111-1111-1111-111111111111"
            },
            "depth": 1
          },
          {
            "group": {
              "id": "11111111-1111-1111-1111-111111111111",
              "group_type": "tenant",
              "name": "T1",
              "tenant_id": "11111111-1111-1111-1111-111111111111"
            },
            "depth": 2
          }
        ]
      }
    ]
  }
}
```

## Примечание по совместимости

`rg/openapi.yaml` и `rg/migration.sql` расходятся по именам полей. В этом файле примеры ориентированы на новую схему БД:
- `parents` -> `ancestors`
- `type_code` -> `group_type`
- `reference_type/reference_id` -> `resource_type/resource_id`
- в сущностях добавлен обязательный `tenant_id`


# Pagination and filters
 Где в модулях есть и пагинация, и фильтры вместе (документация/контракт)

  - mini-chat (доки + OpenAPI, кода модуля пока нет):
      - Контракт GET /v1/chats/{id}/messages с limit/cursor/$select/$orderby/$filter: DESIGN.md:452 (/modules/
        mini-chat/docs/DESIGN.md:452)
      - OpenAPI параметры limit/cursor/$select/$orderby/$filter: openapi.json:1003 (/modules/mini-chat/docs/
        openapi.json:1003)
      - PageInfo/MessagesPage: openapi.json:1608 (/modules/mini-chat/docs/openapi.json:1608)

  Общая документация по пагинации в проекте

  - Базовый общий стандарт курсорной пагинации + OData фильтры/сортировка: guidelines/DNA/REST/QUERYING.md:1 (/
    guidelines/DNA/REST/QUERYING.md:1)
  - REST guideline с отсылкой на querying: guidelines/DNA/REST/API.md:95 (/guidelines/DNA/REST/API.md:95)
  - ModKit-гайд по OData/$select/пагинации: 07_odata_pagination_select_filter.md:1 (/docs/
    modkit_unified_system/07_odata_pagination_select_filter.md:1)

  Что важно

  - В нескольких openspec-доках внутри модулей ссылка на guidelines/DNA/REST/PAGINATION.md, но такого файла нет; актуальный файл: guidelines/DNA/
    REST/QUERYING.md.
      - simple-user-settings project.md:70 (/modules/simple-user-settings/openspec/project.md:70)
      - types-registry project.md:136 (/modules/system/types-registry/openspec/project.md:136)