## Configuration file

`settings.yml` file will be read from run directory (cwd).

The following options are supported:
| Option | Default | Description |
| :-: | :-: | - |
| mongodb_uri | mongodb://localhost:27017 | Database URI to connect to |
| mongodb_db | knowmark | Name of the database that will be used for storage |
| public_content | ./public | Path to built frontend files |
| admin_usernames | \['admin'] | List of top level administrator account usernames |

## Environment variables

Basic configuration can be done through environment variables.
`.env` file will be loaded from server run directory.

Exposed variables are:
| Variable | Default | Description |
| :-: | :-: | - |
| MONGODB_URI | mongodb://localhost:27017 | Database URI to connect to |
| MONGODB_DB_NAME | knowmark | Name of the database that will be used for storage |
| PUBLIC_CONTENT_PATH | ./public | Path to built frontend files |
