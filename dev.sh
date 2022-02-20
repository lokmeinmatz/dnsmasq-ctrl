docker build -t dnsmasq-ctrl .
docker run -p 8080:80 -p 8053:53/tcp -p 8053:53/udp -v /etc/dnsmasq.conf:/etc/dnsmasq.conf:ro dnsmasq-ctrl