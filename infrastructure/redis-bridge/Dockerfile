FROM python:3.11-slim

WORKDIR /app

# Install dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy the bridge script
COPY redis_bridge.py .

# Create a non-root user
RUN useradd -r -s /bin/false app
RUN chown -R app:app /app
USER app

CMD ["python", "redis_bridge.py"] 