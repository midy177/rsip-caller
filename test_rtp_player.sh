#!/bin/bash

# RTP Player Test Script

echo "=== RTP Player Test Script ==="
echo "This script demonstrates how to use the RTP player with different media types."
echo ""

# Create test directories if they don't exist
mkdir -p test_media

# Check if test media files exist, create dummy files if needed
if [ ! -f "test_media/test_audio.wav" ]; then
    echo "Creating dummy WAV file for testing..."
    # Create a minimal WAV header (44 bytes) + some silence
    printf "RIFF\x24\x08\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x01\x00\x40\x1f\x00\x00\x80\x3e\x00\x00\x02\x00\x10\x00data\x00\x08\x00\x00" > test_media/test_audio.wav
    # Add 160 bytes of silence (20ms at 8kHz, 8-bit, mono)
    dd if=/dev/zero bs=160 count=1 >> test_media/test_audio.wav 2>/dev/null
fi

if [ ! -f "test_media/test_video.ivf" ]; then
    echo "Creating dummy IVF file for testing..."
    # Create a minimal IVF header (32 bytes)
    printf "DKIF\x00\x00\x20\x00VP80\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00" > test_media/test_video.ivf
    # Add a dummy frame header (12 bytes)
    printf "\x0c\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00" >> test_media/test_video.ivf
    # Add some dummy VP8 data (minimal)
    printf "\x10\x00\x00\x9d\x01\x2a" >> test_media/test_video.ivf
fi

echo "Test media files created/verified."
echo ""

# Show example usage
echo "=== Example Usage ==="
echo ""

echo "1. Audio playback example:"
echo "cargo run --example rtp_play_example -- --file test_media/test_audio.wav"
echo ""

echo "2. Video playback example:"
echo "cargo run --example rtp_play_example -- --file test_media/test_video.ivf --media-type video"
echo ""

echo "3. The program will:"
echo "   - Generate a local SDP"
echo "   - Wait for you to paste a remote SDP"
echo "   - Start playing the media file to the remote endpoint"
echo ""

echo "=== SDP Example ==="
echo ""
echo "You can use this SDP for testing with ffplay:"
echo "v=0"
echo "o=- 0 0 IN IP4 127.0.0.1"
echo "s=RustRTC RTP Example"
echo "c=IN IP4 127.0.0.1"
echo "t=0 0"
echo "m=audio 5004 RTP/AVP 0"
echo "a=rtpmap:0 PCMU/8000"
echo ""
echo "Or for video:"
echo "v=0"
echo "o=- 0 0 IN IP4 127.0.0.1"
echo "s=RustRTC RTP Example"
echo "c=IN IP4 127.0.0.1"
echo "t=0 0"
echo "m=video 5004 RTP/AVP 96"
echo "a=rtpmap:96 VP8/90000"
echo ""

echo "=== Testing with FFplay ==="
echo "To test reception with ffplay:"
echo "1. Run the RTP player with audio file"
echo "2. Save the generated SDP to a file"
echo "3. Run: ffplay -protocol_whitelist file,udp,rtp -i sdp_file.sdp"
echo ""

echo "=== Notes ==="
echo "- The implementation is simplified for demonstration"
echo "- In production, you would need proper media encoding/decoding"
echo "- Network configuration may be needed for NAT traversal"
echo "- Test files are minimal and may not play correctly in all scenarios"