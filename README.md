# dnsmasq-ctrl
A docker container running dnsmasq and a simple web ui for managing it and seening stats

## configurable

### env vars and their defaults


DNSMASQ_PORT=53


## dev commands 

read .env file:
`set -o allexport && source .env && set +o allexport`