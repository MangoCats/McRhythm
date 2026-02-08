FROM ghcr.io/mtg/essentia:latest

COPY server.py /app/server.py

WORKDIR /app

# Music files mounted at /music (read-only)
VOLUME /music

EXPOSE 5780

CMD ["python3", "server.py"]
