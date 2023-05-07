### General

| Method | Route           |      Status      | Description                                 |
| :----: | :-------------- | :--------------: | :------------------------------------------ |
|  GET   | `/openapi.json` | [✅](#status-map) | Serves OpenAPI v3 specification for the API |

### User management routes

| Method | Route        |      Status      | Description                                            |
| :----: | :----------- | :--------------: | :----------------------------------------------------- |
|  POST  | `/login`     | [🚀](#status-map) | Login form submission                                  |
|  GET   | `/user`      | [💡](#status-map) | Paged list of users                                    |
|  POST  | `/user`      | [🚀](#status-map)️️ | Create a user from [submitted form](#user-create-form) |
|  GET   | `/user/<id>` | [🚀](#status-map)️️ | Query information about user with `<id>`               |
| DELETE | `/user/<id>` | [🚀️](#status-map)️ | Delete user with `<id>`                                |

### Quiz management routes

| Method | Route                   |      Status      | Description                                                    |
| :----: | :---------------------- | :--------------: | :------------------------------------------------------------- |
|  GET   | `/quiz`                 | [💡](#status-map) | Paged list of quizzes                                          |
|  POST  | `/quiz`                 | [🚀](#status-map) | Create a Quiz from JSON data.                                  |
|  GET   | `/quiz/<id>`            | [🚀](#status-map) | Information about quiz with `<id>`                             |
|  POST  | `/quiz/<id>`            | [💡](#status-map) | Submit status updates for quiz with `<id>`                     |
| DELETE | `/quiz/<id>`            | [🚀](#status-map) | Delete quiz with `<id>`                                        |
|  GET   | `/quiz/<id>/<question>` | [💡](#status-map) | Frontend & information about `<question>` for quiz with `<id>` |
|  POST  | `/quiz/<id>/<question>` | [💡](#status-map) | Solution submission for `<question>` for quiz with `<id>`      |
| DELETE | `/quiz/<id>/<question>` | [💡](#status-map) | Remove `<question>` from quiz with `<id>`                      |

### Status map

| Icon  | Meaning                 |
| :---: | :---------------------- |
|   ✅   | Fully implemented       |
|   🚀️   | Backend implemented     |
|   🧩   | All pieces are in place |
|   💡   | Planned                 |
