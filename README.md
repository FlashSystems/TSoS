# TSOS - The source of secrets

## Brief

Sometimes you need to put passwords and other sensitive information into configuration files. This can be done in many ways, most are unsafe and not very intuitive to use. The simplest way is, to put the password directly into the configuration file and make in readable only by the users and groups that absolutely need it. This approach has some drawbacks:

- The secret information can not be managed automatically.
- The secret information gets backuped or the other configuration information will not be backuped.
- The secret informatino will be written to disk. If the disks are not encrypted the information might be leaked when the server is decomissioned.

To solve these drawbacks, complicated mazes of in-memory filesystems, symbolic links and pre-start-scripts are often used. The scripts take a template configuration file and insert the security critical information before starting the service. That way some of the drawbacks can be mitigated. But it makes the management of the configuration files more complicated (the configured config file is not equivalent with the template file) and you have to take special precautions to not include the security critical configuration into backup by accident.

TSOS is here to add another approach to the mix and solve some of the problems. It is surely not usable for every use-case but it gives you another choice for managing secrent configuration information.

### How TSOS works

TSOS uses the container infrastructure of the Linux Kernel to create an overlay for configuration files hat is only visible for one specific process. The configuration file stays at it normal location. TSOS will hand it over to an external process for processing. The processed file - containing the secrets - will be stored within an in-memory filesystem and will never be written to disk. TSOS copies mode bits/ACLs and ownership information to the in-memory file to make sure it has the same security properties as the source configuration file.

TSOS uses an overlay mount to shadow the source configuration file with the processed version. Mount namespaces make sure that only the process launched by TSOS can see the processed file.

## Configuration

TSOS is configured via a TOML configuration file. The files name is the only parameter of the `tsos` executable. All other parameters get passed on to the process launched by TSOS. This makes it possible to use a TSOS configuration file in combination with an appropriate Shebang (`#!`) as a wrapper for any executable.

The configuration file accepts the following parameters without any preceeding section:

| Parameter | Description | Mandatory |
|-----------|-------------|-----------|
| exec      | Absolute path to the executable that should be launched by tsos after preparig all configuration files. | yes |
| search_path | A TOML array of paths that should be searched to find a secret provider. | no |
| uid       | UID to use when starting the program specified by `exec`. The user ID can be specified as a nummeric value or a user name. If this parameter is missing the program will be run as root. | no |
| gid       | Group to use when starting the program specified by `exec`. The group ID can be specified as a nummeric value or a group name. If this parameter is missing the program will be run as root. | no |

The files that should be processed by TSOS are listed within the `secrets` section. The secret provider to use is listed as the key. The files that should be processed by this secret provider are listed as an array of file names. Eventho the file names can be listed as relative path names it is not recommended to do so.

```toml
exec=/bin/myserver
uid=msrv
gid=msrv

[secrets]
pw-provider=
	"/etc/myserver.conf"
]
```

This example configuration file passes the file `/etc/myserver.conf` to the secret provider `pw-provider`. After the file was successfully processed  and overlayed TSOS starts the program `/bin/myserver` as the user `msrv` and group `msrv` (???).

## Locating a secret provider

Secret providers are executable programms or scripts that accept the source file (the template) as the first and the destination file (the target) as the second parameter. TSOS searches different locations for an executable file that has the name of the secret provider. The following locations are searched in the specified order:

1. The directories specified in the `search_path` configuration option. The directories are searched from left to right.
2. The search path specified in the `TSOS_PATH` environment variable. Multiple paths are searched from left to right.
3. The hard coded path `/etc/tsos.d`
4. The hard coded path `/usr/lib/tsos`

As soon as an executable with the name specified as the key in the `secrets` section is found the search stops and the found executable is used.

### Environment variables

## Loging and debugging

## Creating a secret provider

## Usage with systemd

## Building TSOS

## Reporting bugs

