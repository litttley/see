
# sws

sws is a static web server, developed using rust.

## Features

## Install

## Config

Use `yaml` format as a configuration file, You can use `sws -c /your/config.yml` to specify the configuration file location.

Complete configuration file example: 

```yaml
- server:
    host: domain.com      # Domain name to be bound
    listen: 80            # Port to be monitored
    root: /root/www       # Directory that requires service
    gzip: true            # Whether to open Gzip
    index: index.html     # Index file
    directory: true       # Whether to display the file list
    headers:              # Header in response
      - auth 12345
      - Set-Cookie 12345
    extensions:           # Sets file extension fallbacks
      - html
      - htm
    error:                # Custom error page
      404: 404.html
      500: 500.html
    log:                  # Log save location
      error: /logs/domain.error.log
      success: /logs/domain.success.log
# More server ...
```

## Todo

* [x] Custom header
* [x] Extensions
* [ ] Parse request `50%`
* [x] Custom error
* [ ] Async
* [ ] Log
* [ ] Gzip
* [ ] Proxy
* [ ] HTTPS