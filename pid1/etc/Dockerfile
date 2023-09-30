FROM alpine

RUN apk update && apk add coreutils procps

COPY simple /simple

COPY zombie /usr/bin/zombie

COPY sigterm_handler /usr/bin/sigterm_handler

COPY sigterm_loop /usr/bin/sigterm_loop

CMD ["/simple"]
