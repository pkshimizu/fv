# 開発・動作確認用の Linux 環境。配布用ではなくソースは焼き込まず、
# docker-compose.yml の bind mount で渡す（コード編集が即反映される）。
#
# ベースは公式 rust:slim（Debian, stable）。fv は edition 2024 のため
# 新しめの stable が必要で、CI の dtolnay/rust-toolchain@stable と方針を合わせる。
FROM rust:slim

# Linux ビルドに必要なシステム依存。
# - libasound2-dev: rodio -> alsa-sys が要求する ALSA 開発ヘッダ（CI と同じ）
# - pkg-config / build-essential: *-sys クレートのビルドに使われる
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libasound2-dev \
        pkg-config \
        build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /work
