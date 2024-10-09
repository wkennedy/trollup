# Use a slim Debian image for a smaller footprint
FROM debian:trixie-slim

# Install OpenSSL and CA certificates
# These are often required by Rust SSL/TLS libraries
RUN apt-get update && \
    apt-get install -y openssl ca-certificates libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Create a non-root user to run the application
RUN useradd -ms /bin/bash trollup

# Copy the pre-built binary into the image
COPY ./target/release/trollup-api /usr/local/bin/trollup-api

# Set the ownership of the binary to the non-root user
RUN chown trollup:trollup /usr/local/bin/trollup-api

# Switch to the non-root user
USER trollup

# Set the startup command to run your binary
CMD ["trollup-api"]