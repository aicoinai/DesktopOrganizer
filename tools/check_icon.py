import pefile, os

exe_path = r"D:\WorkSpace\DesktopOrganizer\release\DesktopOrganizer.exe"
pe = pefile.PE(exe_path)

print("=== Resource Directory Entries ===")
if hasattr(pe, 'DIRECTORY_ENTRY_RESOURCE'):
    for i, entry in enumerate(pe.DIRECTORY_ENTRY_RESOURCE.entries):
        try:
            name = entry.struct.name
            if name == 3:
                print(f"  [{i}] RT_ICON (id=3) - FOUND!")
                for j, e2 in enumerate(entry.directory.entries):
                    print(f"    Sub #{j}: offset=0x{e2.data.struct.offsetToData:x}, size={e2.data.struct.size}")
        except Exception as e:
            print(f"  [{i}] Exception: {e}")

print(f"\nFile size: {os.path.getsize(exe_path):,} bytes")

# Check raw import directory
print("\n=== IMAGE_DIRECTORY_ENTRY_RESOURCE ===")
try:
    rvadir = pe.get_data_directory(pefile.DIRECTORY_ENTRY['IMAGE_DIRECTORY_ENTRY_RESOURCE'])
    print(f"RVA: 0x{rvadir.VirtualAddress:x}, Size: {rvadir.Size}")
except Exception as e:
    print(f"Error: {e}")
