class Skillsdash < Formula
  desc "Cross-platform TUI for managing AI skills across Claude and Agents providers"
  homepage "https://github.com/saeedvaziry/skillsdash"
  version "0.3.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "b3d88a46222f7c1e94d6be311731403072008602c4ce8b53c4f96fb9edc08091"
    end
    on_intel do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "3bc263ca0492732e61f0abe7eeb3f474aeb08b54383b49b06e1dde87ecedd4a1"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "59926f57b1002ad3bf56c17482842ef48458f697c8dcecc6661070df52550b00"
    end
    on_intel do
      url "https://github.com/saeedvaziry/skillsdash/releases/download/v#{version}/skillsdash-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "cdde12e59cb77ab69430a3ca9e40b73ffc778da33937060edd4bc53edd66882b"
    end
  end

  def install
    bin.install "skillsdash"
  end

  test do
    assert_match "skillsdash", shell_output("#{bin}/skillsdash --version", 2)
  end
end
