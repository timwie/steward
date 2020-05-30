#!/bin/bash
echo "How do you want to update the controller?"
echo "  1) Download latest binary"
echo "  2) Select version, and download binary"
echo "  3) Build development version"
echo ""

read n
echo ""

case $n in
  1) cd /tmp/
     git clone https://github.com/timwie/steward.git
     cd steward/
     v=$(git describe --abbrev=0)
     wget -O /home/steward/steward-x86_64-unknown-linux-gnu https://github.com/timwie/steward/releases/download/$v/steward-x86_64-unknown-linux-gnu
     ;;

  2) echo "Enter the version you want (f.e. '0.1.0'): "
     read v
     echo ""
     wget -O /home/steward/steward-x86_64-unknown-linux-gnu https://github.com/timwie/steward/releases/download/v$v/steward-x86_64-unknown-linux-gnu
     ;;

  3) mkdir -p /home/steward/.repo
     cd /home/steward/.repo
     git -C steward pull origin master || git clone https://github.com/timwie/steward.git
     cd steward/
     cargo build --target x86_64-unknown-linux-gnu
     mv target/x86_64-unknown-linux-gnu/debug/steward /home/steward/steward-x86_64-unknown-linux-gnu
     ;;

  *) echo "invalid option";;
esac
