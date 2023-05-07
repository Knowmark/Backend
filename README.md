# Knowmark Backend

Application I chose to write for my Dynamic Web Applications class.

The purpose of this project is providing a versatile quiz server which allows authors (i.e. teachers) to publish graded
quizzes and users (i.e. students) to solve them. Server then automatically validates submitted solutions where
possible (based on several different validation methods), allows quiz author to manually grade ungraded answers, modify
grades on incorrectly graded answers, and finally send out grading results in bulk and generate a report.

## Building

To build the server, clone the repo and run [cargo](https://doc.rust-lang.org/cargo/):

```sh
cargo build --release --bin knowmark-server
```

The resulting binary will be located in: `./target/<your-arch-quad>/release/knowmark-server`

NOTE: Crosscompilation from Linux to Windows with gnu target works fine, msvc wasn't tested.

## Initial Setup

Requires a running MongoDB instance.

> [MongoDB Docs: Installation](https://www.mongodb.com/docs/manual/administration/install-community/)

By default, a localhost instance will be used, this can be changed through [configuration file](./docs/Running.md#configuration-file) or [environment variables](./docs/Running.md#environment-variables)

## Functionality

Endpoints will likely change as the application is being developed.

For a quick listing of the routes, please refer to:

- [API Endpoints - v1](./docs/Endpoints.md)

Security features are documented in [docs/Security.md](./docs/Security.md).

## Testing

Long term goal is to have all backend routes unit tested.

## License

This project is licensed under GPLv2 license.
A copy of the license is provided in the [LICENSE.md](LICENSE.md) file.
