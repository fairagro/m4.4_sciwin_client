import os

def create_directory_and_files(directory: str, files: dict):
    os.makedirs(directory, exist_ok=True) 
    
    for filename, content in files.items():
        file_path = os.path.join(directory, filename)
        with open(file_path, 'w') as f:
            f.write(content)

if __name__ == "__main__":
    dir_name = "my_directory"
    files_content = {
        "a.txt": "a",
        "b.txt": "b",
        "c.txt": "c"
    }
    
    create_directory_and_files(dir_name, files_content)
