# TSOS - The source of secrets

## Brief

Sometimes you need to put passwords and other sensitive information into configuration files. This can be done in many ways, most are unsafe and not very intuitive to use. The simplest way is, to put the password directly into the configuration file and make in readable only by the users and groups that absolutely need it. This approach has some drawbacks:

- The secret information can not be managed automatically.
- The secret information gets backuped or the other configuration information will not be backuped.
- The secret informatino will be written to disk. If the disks are not encrypted the information might be leaked when the server is decommissioned.

To solve these drawbacks, complicated mazes of in-memory file-systems, symbolic links and pre-start-scripts are often used. The scripts take a template configuration file and insert the security critical information before starting the service. That way some of the drawbacks can be mitigated. But it makes the management of the configuration files more complicated (the configured file is not equivalent with the template file) and you have to take special precautions to not include the security critical configuration into your backup by accident.

TSOS is here to add another approach to the mix and solve some of the problems. It is surely not usable for every use-case but it gives you another choice for managing secret configuration information.

### How TSOS works

TSOS uses the container infrastructure of the Linux Kernel to create an overlay for configuration files hat is only visible for one specific process (and its children). The configuration file stays at it normal location. TSOS will hand it over to an external program or script for processing. The processed file - containing the secrets - will be stored within an in-memory filesystem and will never be written to disk. TSOS copies mode bits/ACLs and ownership information to the in-memory file to make sure it has the same security properties as the source configuration file.

TSOS uses an overlay mount to shadow the source configuration file with the processed version. Mount namespaces make sure that only the process launched by TSOS can see the processed file.

## Configuration

TSOS is configured via a TOML configuration file. The files name is the only parameter of the `tsos` executable. All other parameters get passed on to the process launched by TSOS. This makes it possible to use a TSOS configuration file in combination with an appropriate Shebang (`#!`) as a wrapper for any executable.

The configuration file accepts the following parameters without any preceeding section:

| Parameter | Description | Mandatory |
|-----------|-------------|-----------|
| `exec`    | Absolute path to the executable that should be launched by tsos after preparig all configuration files. | yes |
| `search_path` | A TOML array of paths that should be searched to find a secret provider. | no |
| `env_path`  | Enable searching for secret providers within the paths specified by the `TSOS_PATH` environment variable. | no |
| `uid`       | UID to use when starting the program specified by `exec`. The user ID can be specified as a nummeric value or a user name. If this parameter is missing the program will be run as root. | no |
| `gid`       | Group to use when starting the program specified by `exec`. The group ID can be specified as a nummeric value or a group name. If this parameter is missing the primary group of the user supplied by the `uid` parameter will be used. If no `uid` parameter is supplied, the group will be set to root. | no |

The files that should be processed by TSOS are listed within the `secrets` section. The secret provider to use is listed as the key. The files that should be processed by this secret provider are passed as an array of file names. The file names can be listed as relative path names, but it is not recommended to do so.

```toml
exec=/bin/myserver
uid=msrv
gid=msrv

[secrets]
pw-provider= [
	"/etc/myserver.conf"
]
```

This example configuration file passes the file `/etc/myserver.conf` to the secret provider `pw-provider`. After the file was successfully processed and overlayed TSOS starts the program `/bin/myserver` as the user `msrv` and group `msrv`.

## Locating a secret provider

Secret providers are executable programs or scripts that accept the source file (the template) as the first and the destination file (the target) as the second parameter. TSOS searches different locations for an executable file that has the name of the secret provider. The following locations are searched in the specified order:

1. The directories specified in the `search_path` configuration option. The directories are searched from left to right.
2. If the configuration file set `env_path` to `true`: The search path specified in the `TSOS_PATH` environment variable. Multiple paths are searched from left to right.
3. The hard coded path `/etc/tsos.d`
4. The hard coded path `/usr/lib/tsos`

The name of the secret provider is equivalent to the key within the `secrets` section of the TOML configuration file. As soon as an executable with a matching name is found the search stops and the found executable is executed. The secret providers are always launched as the user that has started the `tsos` executable. If TSOS is run as `root` all secret providers will run as root as well.

> **WARNING**: Make sure all paths that are searched for secret providers are _not_ writable for unprivileged users.

### Environment variables

Because TSOS can be used as a direct wrapper for an executable there is no way to specify command line options. Therefore TSOS uses environment variables to allow some configuration options to be set.

| Environment variable | Description |
|----------------------|-------------|
| `TSOS_PATH`            | Search path for secret providers. Multiple paths must be separated by a colon (`:`). The syntax is equivalent to the `PATH` environment variable. |
| `TSOS_LOG`             | The requested log level. See Chapter "Logging and debugging". |

Due to security considerations the `TSOS_PATH` environment variable is only honored if `env_path` is set to `true` within the configuration file.

## Logging and debugging

TSOS normally runs with the log level `warn` enabled. To get more information output the environment variable `TSOS_LOG` can be set to one of the following values:

| Log level | Description |
|-----------|-------------|
| `error`   | Output only errors. |
| `warn`    | Output errors and warnings. |
| `info`    | Output errors, warnings and informational messages. |
| `debug`   | Output all messages. |

The `debug` log level outputs a vast amount of information and should only be used for diagnostic purposes.

## Creating a secret provider

A secret provider is an executable or script hat transforms a template file into the final file used by the process started by TSOS. The secret provider gets two command line argument:

1. The template (input) file as specified within the configuration TOML file.
2. The target (output) file. This file already exists (it is empty) and must be overwritten (or appended) by the secret provider.

TSOS will make sure that mode-bits/ACLs and ownership information are copied to the target file as soon as the secret provider returns.

The secret provider is run as the user that starts TSOS. No privileges are dropped when running the secret provider. The secret provider can do anything the user running TSOS can do. The only exception is mounting file systems. The secret provider is run with the mount namespace isolation already enabled and mounts done by a secret provider will _not_ be visible to the outside world.

## Usage with systemd

TSOS is by default build with systemd integration. It uses the `JOURNAL_STREAM` environment variable (see (system.exec)[https://www.freedesktop.org/software/systemd/man/systemd.exec.html#%24JOURNAL_STREAM]) to detect if TSOS is startet as a systemd unit. If that's the case logging is automatically switched to systemd logging. That way journald metadata is automatically added to the log messages.

If systemd is used to start a TSOS controlled service the `tsos` executable must be launched as root. Any configured users and groups (via `User=` or `Group=`) must be migrated into the TSOS configuration file.

If Capabilites of the process should be restricted the capabilities necessary for TSOS to work must be kept active:

- CAP_SETGID
- CAP_SETUID
- CAP_SYS_ADMIN

> *NOTE*: It is planned that TSOS can drop the capabilities that are necessary only for its operation before launching the process. Currently this not _not_ implemented.

## Building TSOS

To build TSOS you need rust 1.37 and cargo. Just clone the git repository and execute `cargo build --release` to build TSOS.

### Build options

TSOS enables all of its features by default. If you do not want to build a specific feature you have to disable all default features via `--no-default-features` and re-enable the features that should be built via `--features`. Most features will add additional dependencies to the `tsos` executable. They are listed in the `Dependencies` column of the features table.

Currently the following features are available:

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| acl     | Enable support for file system ACLs. If this feature is disabled only mode bits will be copied to the target file. | libacl |
| systemd | Enable support for journal logging. If this feature is enabled tsos will try to autodetect systemd and use journald based logging if it is started as a systemd unit. | libsystemd |

### Test suite

TSOS comes with a test suite that tries to exercise as much of its functionality as possible. As TSOS is a very low level tool, most of the functionality unfortunately requires root privileges. TSOS will use `sudo` to acquire root privileges before running the tests.

> **WARNING**: Because tests can go wrong there is a risk of TSOS damaging your linux installation while performing the test suite as the root user. It is recommended to use a virtual machine for running the tests.

To run the test suite open a terminal, switch to the root directory of the TSOS repository and execute `cargo test`.

## Reporting bugs
