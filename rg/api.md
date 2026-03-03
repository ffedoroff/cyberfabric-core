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

`GET /api/resource-group/v1/types?limit=50&cursor=<opaque>&$filter=contains(code,'ten')&$orderby=code asc&$select=code,ancestors`
`POST /api/resource-group/v1/types`

```jsonc
{
  // Request (GET list)
  "request": {
    "limit": 50,
    "cursor": null,
    "$filter": "contains(code,'ten')",
    "$orderby": "code asc",
    "$select": "code,ancestors"
  },
  // Response (GET list)
  "response": {
    "items": [
      {
        "code": "department",
        "ancestors": ["tenant"]
      },
      {
        "code": "tenant",
        "ancestors": []
      }
    ],
    "page_info": {
      "limit": 50,
      "next_cursor": null,
      "prev_cursor": null
    }
  }
}
```

## 2) `/api/resource-group/v1/types/{code}`

`GET /api/resource-group/v1/types/department?$select=code,ancestors`
`PUT /api/resource-group/v1/types/department`
`DELETE /api/resource-group/v1/types/department`

```jsonc
{
  // Request (GET single)
  "request": {
    "$select": "code,ancestors"
  },
  // Response (GET single)
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
  // Request (POST create)
  "request": {
    "group_type": "department",
    "name": "D2",
    "parent_id": "11111111-1111-1111-1111-111111111111",
    "tenant_id": "11111111-1111-1111-1111-111111111111",
    "external_id": "D2"
  },
  // Response (POST create)
  "response": {
    "id": "22222222-2222-2222-2222-222222222222",
    "parent_id": "11111111-1111-1111-1111-111111111111",
    "group_type": "department",
    "name": "D2",
    "tenant_id": "11111111-1111-1111-1111-111111111111",
    "external_id": "D2",
    "created": "2026-02-25T12:00:00.000Z",
    "modified": null
  }
}
```

## 4) `/api/resource-group/v1/groups/{id}`

`GET /api/resource-group/v1/groups/22222222-2222-2222-2222-222222222222?$select=id,parent_id,group_type,name,tenant_id,external_id`
`PUT /api/resource-group/v1/groups/22222222-2222-2222-2222-222222222222`
`DELETE /api/resource-group/v1/groups/22222222-2222-2222-2222-222222222222?force=true|false`

```jsonc
{
  // Request (GET single)
  "request": {
    "$select": "id,parent_id,group_type,name,tenant_id,external_id"
  },
  // Response (GET single)
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

## 4.1) `/api/resource-group/v1/groups` (pagination + filters)

`GET /api/resource-group/v1/groups?id.in=22222222-2222-2222-2222-222222222222,22222222-2222-2222-2222-222222222221,22222222-2222-2222-2222-222222222224&tenant_id=11111111-1111-1111-1111-111111111111&limit=50&cursor=<opaque>&$orderby=name asc&$select=id,name,group_type,tenant_id`

```jsonc
{
  // Request (GET list): аналог нескольких GET /groups/{id}
  "request": {
    "id.in": [
      "22222222-2222-2222-2222-222222222222",
      "22222222-2222-2222-2222-222222222221",
      "22222222-2222-2222-2222-222222222224"
    ],
    "tenant_id": "11111111-1111-1111-1111-111111111111",
    "limit": 50,
    "cursor": null,
    "$filter": null,
    "$orderby": "name asc",
    "$select": "id,name,group_type,tenant_id"
  },
  // Response (GET list)
  "response": {
    "items": [
      {
        "id": "22222222-2222-2222-2222-222222222222",
        "name": "D2",
        "group_type": "department",
        "tenant_id": "11111111-1111-1111-1111-111111111111"
      }
    ],
    "page_info": {
      "limit": 50,
      "next_cursor": null,
      "prev_cursor": null
    }
  }
}
```

## 5) `/api/resource-group/v1/groups/{id}/ancestors`

`GET /api/resource-group/v1/groups/33333333-3333-3333-3333-333333333333/ancestors?limit=50&cursor=<opaque>&$orderby=depth asc&$select=group.id,group.name,group.group_type,depth`

```jsonc
{
  // Request (GET list)
  "request": {
    "limit": 50,
    "cursor": null,
    "$orderby": "depth asc",
    "$select": "group.id,group.name,group.group_type,depth"
  },
  // Response (GET list)
  "response": {
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
    ],
    "page_info": {
      "limit": 50,
      "next_cursor": null,
      "prev_cursor": null
    }
  }
}
```

## 6) `/api/resource-group/v1/groups/{id}/descendants`

`GET /api/resource-group/v1/groups/11111111-1111-1111-1111-111111111111/descendants?limit=50&cursor=<opaque>&$orderby=depth asc&$select=group.id,group.name,group.group_type,depth`

```jsonc
{
  // Request (GET list)
  "request": {
    "limit": 50,
    "cursor": null,
    "$orderby": "depth asc",
    "$select": "group.id,group.name,group.group_type,depth"
  },
  // Response (GET list)
  "response": {
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
    ],
    "page_info": {
      "limit": 50,
      "next_cursor": null,
      "prev_cursor": null
    }
  }
}
```

## 7) `/api/resource-group/v1/groups/{id}/references`

`GET /api/resource-group/v1/groups/33333333-3333-3333-3333-333333333333/references?limit=50&cursor=<opaque>&$filter=resource_type eq 'resource'&$orderby=resource_id asc&$select=resource_type,resource_id,tenant_id`
`POST /api/resource-group/v1/groups/33333333-3333-3333-3333-333333333333/references`
`DELETE /api/resource-group/v1/groups/33333333-3333-3333-3333-333333333333/references`

```jsonc
{
  // Request (GET list)
  "request": {
    "limit": 50,
    "cursor": null,
    "$filter": "resource_type eq 'resource'",
    "$orderby": "resource_id asc",
    "$select": "resource_type,resource_id,tenant_id"
  },
  // Response (GET list)
  "response": {
    "items": [
      {
        "resource_type": "resource",
        "resource_id": "R4",
        "tenant_id": "11111111-1111-1111-1111-111111111111"
      }
    ],
    "page_info": {
      "limit": 50,
      "next_cursor": null,
      "prev_cursor": null
    }
  }
}
```

## 8) `/api/resource-group/v1/groups/descendants` (batch read via filters)

`GET /api/resource-group/v1/groups/descendants?$filter=source_group_id in (11111111-1111-1111-1111-111111111111,22222222-2222-2222-2222-222222222222)&limit=50&cursor=<opaque>&$orderby=group.id asc&$select=group.id,group.name,depth`

```jsonc
{
  // Request (GET list)
  "request": {
    "$filter": "source_group_id in (11111111-1111-1111-1111-111111111111,22222222-2222-2222-2222-222222222222)",
    "limit": 50,
    "cursor": null,
    "$orderby": "group.id asc",
    "$select": "group.id,group.name,depth"
  },
  // Response (GET list)
  "response": {
    "items": [
      {
        "source_group_id": "11111111-1111-1111-1111-111111111111",
        "group": {
          "id": "22222222-2222-2222-2222-222222222222",
          "group_type": "department",
          "name": "D2",
          "tenant_id": "11111111-1111-1111-1111-111111111111"
        },
        "depth": 1
      },
      {
        "source_group_id": "11111111-1111-1111-1111-111111111111",
        "group": {
          "id": "33333333-3333-3333-3333-333333333333",
          "group_type": "branch",
          "name": "B3",
          "tenant_id": "11111111-1111-1111-1111-111111111111"
        },
        "depth": 2
      }
    ],
    "page_info": {
      "limit": 50,
      "next_cursor": null,
      "prev_cursor": null
    }
  }
}
```

## 9) `/api/resource-group/v1/groups/ancestors` (batch read via filters)

`GET /api/resource-group/v1/groups/ancestors?$filter=source_group_id in (33333333-3333-3333-3333-333333333333,77777777-7777-7777-7777-777777777777)&limit=50&cursor=<opaque>&$orderby=group.id asc&$select=group.id,group.name,depth`

```jsonc
{
  // Request (GET list)
  "request": {
    "$filter": "source_group_id in (33333333-3333-3333-3333-333333333333,77777777-7777-7777-7777-777777777777)",
    "limit": 50,
    "cursor": null,
    "$orderby": "group.id asc",
    "$select": "group.id,group.name,depth"
  },
  // Response (GET list)
  "response": {
    "items": [
      {
        "source_group_id": "33333333-3333-3333-3333-333333333333",
        "group": {
          "id": "22222222-2222-2222-2222-222222222222",
          "group_type": "department",
          "name": "D2",
          "tenant_id": "11111111-1111-1111-1111-111111111111"
        },
        "depth": 1
      },
      {
        "source_group_id": "33333333-3333-3333-3333-333333333333",
        "group": {
          "id": "11111111-1111-1111-1111-111111111111",
          "group_type": "tenant",
          "name": "T1",
          "tenant_id": "11111111-1111-1111-1111-111111111111"
        },
        "depth": 2
      }
    ],
    "page_info": {
      "limit": 50,
      "next_cursor": null,
      "prev_cursor": null
    }
  }
}
```

## Примечание по совместимости

`rg/openapi.yaml` и `rg/migration.sql` расходятся по именам полей. В этом файле примеры ориентированы на новую схему БД:
- `parents` -> `ancestors`
- `type_code` -> `group_type`
- `reference_type/reference_id` -> `resource_type/resource_id`
- в сущностях добавлен обязательный `tenant_id`

## Pagination and filters

- Для list/read использовать `GET` + фильтры на коллекции (включая `id.in`), а не `POST` batch-read.
- Для пагинации использовать `limit` и `cursor`.
- Для OData использовать `$filter`, `$orderby`, `$select`.
- Документация OData `$filter`: https://docs.oasis-open.org/odata/odata/v4.01/odata-v4.01-part1-protocol.html#sec_SystemQueryOptionfilter
- Формат list-ответа: `items` + `page_info`.
