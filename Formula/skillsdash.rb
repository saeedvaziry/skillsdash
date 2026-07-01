class Skillsdash < Formula
  desc "Cross-platform TUI for managing AI skills across Claude and Agents providers"
  homepage "https://github.com/saeedvaziry/skillsdash"
  version "0.2.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "5f3f0f37d9e4f78a91816fb42a0fd8db87374fd8285e0baf9484bc7a7ed3bfea"
    end
    on_intel do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "24d659058a35d5a1c13c9d5271af5f933028861470ef5dfbde7b5bf243ce891b"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "30d5e0c5528299d38681f21a2964bf537fcb68bd716c80eb3df5984351f55b83"
    end
    on_intel do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "2c4b9c6ff9f673242326eb4e62bcb3010c8f16e5de7ceca5a3ea88813c1f9d6a"
    end
  end

  def install
    bin.install "skillsdash"
  end

  test do
    assert_match "skillsdash", shell_output("#{bin}/skillsdash --version", 2)
  end
end
