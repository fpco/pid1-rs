FROM alpine

RUN apk update && apk add gcc musl-dev procps

COPY simple /simple

COPY zombie.c /zombie.c

RUN gcc /zombie.c

CMD ["/simple"]
