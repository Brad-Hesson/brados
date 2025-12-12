cargo build --release
mkdir -p ./boot_drive/efi/boot
cp ./target/x86_64-unknown-uefi/release/brados.efi ./boot_drive/efi/boot/bootx64.efi
qemu-kvm -bios $OVMF_FIRMWARE -drive format=raw,file=fat:rw:./boot_drive