FROM python:3.13.15-slim-bookworm@sha256:ed86c82274b3c69b52fb5820f358f0bd7df0b603332063cb5c6e32bd220c3e6e

COPY LICENSE /usr/share/licenses/flow-like/LICENSE
WORKDIR /opt/object-store
COPY apps/backend/docker-compose/object-store/requirements.txt ./
RUN pip install --no-cache-dir --requirement requirements.txt
COPY apps/backend/docker-compose/object-store/bootstrap.py apps/backend/docker-compose/object-store/conformance.py ./
USER 65532:65532
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1
ENTRYPOINT ["python", "/opt/object-store/bootstrap.py"]
