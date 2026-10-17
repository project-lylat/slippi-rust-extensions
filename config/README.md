# SlippiConfig Module

`slippi-config` holds the Lylat server addresses for each environment. Dolphin uses them when
it creates the EXI device with `slprs_exi_device_create_lylat`; `slprs_exi_device_create`
keeps Slippi's servers.

## Environments

The environment is picked when the library is built, from `SLIPPI_ENV`
(`development` by default):

| Setting      | `development`                      | `production`                  |
|--------------|------------------------------------|-------------------------------|
| GraphQL      | `http://localhost:3000/graphql`    | `https://lylat.gg/graphql`    |
| User API     | `http://localhost:3000/users/slp`  | `https://lylat.gg/users/slp`  |
| Sign-in page | `http://localhost:3000/users/enable` | `https://lylat.gg/users/enable` |
| Update page  | `http://localhost:3000/download`   | `https://lylat.gg/download`   |
| Site (QR sign-in) | `http://localhost:3000`       | `https://lylat.gg`            |
| Matchmaking  | `localhost:43113`                  | `lylat.gg:43113`              |

```sh
SLIPPI_ENV=production cargo build --release
```

Changing `SLIPPI_ENV` rebuilds this crate. The values live in `src/development.rs` and
`src/production.rs`.

## Overrides

These environment variables, read when the game starts, win over the built-in values, so a
URL can be switched without a rebuild:

- `SLIPPI_GRAPHQL_URL`: game reports, match status and rank.
- `SLIPPI_USER_REST_URL`: the account lookup; the client appends `/{uid}?...`.
- `SLIPPI_LOGIN_PAGE_URL`: the sign-in page; the client appends `?path={account file}`.
- `SLIPPI_UPDATE_PAGE_URL`: the page opened when an update is needed.
- `SLIPPI_LYLAT_ORIGIN`: the Lylat site, for the QR sign-in.
- `SLIPPI_MM_HOST`, `SLIPPI_MM_PORT`: the Lylat matchmaking server.

Dolphin reads the site and matchmaking server with `slprs_lylat_origin`,
`slprs_lylat_mm_host` and `slprs_lylat_mm_port`.

`cargo run -p slippi-config` prints the configuration this build uses.
