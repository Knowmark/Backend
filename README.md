# Knowmark Backend

Application I decided to write for my Dynamic Web Applications class. It is also something that will help tremendously
with grading assignments for Computer Networks class where non-standard implementation of regex by Moodle doesn't cut
it.

The purpose of this project is providing a versatile quiz server which allows authors (e.g. teachers) to publish graded
quizzes and users (e.g. students) to solve them. Server then automatically validates submitted solutions where
possible (based on several different validation methods), allows quiz author to manually grade ungraded answers, modify
grades on incorrectly graded answers, and finally send out grading results in bulk and generate a report.

## Routes️

Planned routes will likely change in functionality as they're being developed.

Responses of most of the specified rotes are dependant on information (e.g. headers) provided by requests.

### General

| Method |  Route  | Status | Description |
| :----: | :------ | :----: | :---------- |
| GET    | `/api`  | [🚀](#status-map) | Serves API documentation for frontend |

### User management routes

| Method |       Route      | Status | Description |
| :----: | :--------------- | :----: | :---------- |
| GET    | `/login`         | [💡](#status-map) | Login form frontend |
| POST   | `/login`         | [🚀](#status-map) | Login form submission |
| GET    | `/user`          | [💡](#status-map) | Paged list of users |
| POST   | `/user`          | [🚀](#status-map)️️ | Create a user from [submitted form](#user-create-form) |
| GET    | `/user/<id>`     | [🚀](#status-map)️️ | Query information about user with `<id>` |
| DELETE | `/user/<id>`     | [🚀️](#status-map)️ | Delete user with `<id>` |

### Quiz management routes

| Method |          Route          | Status | Description |
| :----: | :---------------------- | :----: | :---------- |
| GET    | `/quiz`                 | [💡](#status-map) | Paged list of quizzes |
| POST   | `/quiz`                 | [🚀](#status-map) | Create a Quiz from JSON data. |
| GET    | `/quiz/<id>`            | [🚀](#status-map) | Information about quiz with `<id>` |
| POST   | `/quiz/<id>`            | [💡](#status-map) | Submit status updates for quiz with `<id>` |
| DELETE | `/quiz/<id>`            | [🚀](#status-map) | Delete quiz with `<id>` |
| GET    | `/quiz/<id>/<question>` | [💡](#status-map) | Frontend & information about `<question>` for quiz with `<id>` |
| POST   | `/quiz/<id>/<question>` | [💡](#status-map) | Solution submission for `<question>` for quiz with `<id>` |
| DELETE | `/quiz/<id>/<question>` | [💡](#status-map) | Remove `<question>` from quiz with `<id>` |

### Status map

| Icon | Meaning |
| :--: | :------ |
| ✅ | Fully implemented |
| 🖥 | Backend implemented<br>Frontend in development |
| 🚀️ | Backend implemented |
| 🧩 | All pieces are in place |
| 💡 | Planned |

## Forms

### User create form

Form submitted by the client to server which provides server with information required to create a user account.
Password is transmitted in cleartext and relies on [TLS](https://en.wikipedia.org/wiki/Transport_Layer_Security) to
protect end-users from [MITM attacks](https://en.wikipedia.org/wiki/Man-in-the-middle_attack).
Passwords are hashed using [bcrypt](https://en.wikipedia.org/wiki/Bcrypt) with 15 cycles. Only password hashes are stored.

#### Validation

- Username must match regex: `[\w\d_\-.]{5,32}`
- Password must be of length in range: \[8, 50]
    - Max. length restriction will be increased once pre-hashing is implemented.

#### Example

```json
{
  "username": "PineappleMan",
  "password": "IlikeP1neapples!"
}
```

## License

This project is licensed under GPLv2 license.
A copy of the license is provided in the [LICENSE.md](LICENSE.md) file.
