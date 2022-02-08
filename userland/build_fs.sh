#!/bin/sh
cd $(dirname $0)

mkdir -p fs/bin || exit $?

cargo build --release || exit $?

cp target/i586-unknown-linux-gnu/release/cat fs/bin || exit $?
cp target/i586-unknown-linux-gnu/release/ls fs/bin || exit $?
cp target/i586-unknown-linux-gnu/release/shell fs/bin || exit $?

if test ! -f "test_ext2.fs"; then
	echo "Creating new ext2 filesystem"
	dd if=/dev/zero of=test_ext2.fs bs=4096 count=512 || exit $?
	mkfs.ext2 test_ext2.fs || exit $?
fi

./mount_fs.sh || exit $?
sudo rm -rf ./mnt_ext2/* || exit $?
sudo cp -r fs/* ./mnt_ext2 || exit $?
./unmount_fs.sh || exit $?
