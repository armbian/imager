use alloc::string::String;
use alloc::vec::Vec;

pub fn path_check(path: &str, is_goal: &mut bool) -> usize {
    // Iterate through each character and its index in the string
    for (i, c) in path.chars().enumerate() {
        // Check if maximum filename length limit is reached
        if i >= 255 {
            break;
        }

        // Check if character is a path separator
        if c == '/' {
            *is_goal = false;
            return i;
        }

        // Check if end of string is reached
        if c == '\0' {
            *is_goal = true;
            return i;
        }
    }

    // If neither '/' nor '\0' was found, and length is less than maximum filename length
    *is_goal = true;
    path.len()
}

#[cfg(test)]
mod path_tests {
    use super::*;
    #[test]
    fn test_ext4_path_check() {
        let mut is_goal = false;

        // Test root path
        assert_eq!(path_check("/", &mut is_goal), 0);
        assert!(!is_goal, "Root path should not set is_goal to true");

        // Test normal path
        assert_eq!(path_check("/home/user/file.txt", &mut is_goal), 0);
        assert!(!is_goal, "Normal path should not set is_goal to true");

        // Test path without slashes
        let path = "file.txt";
        assert_eq!(path_check(path, &mut is_goal), path.len());
        assert!(is_goal, "Path without slashes should set is_goal to true");

        // Test null character at end of path
        let path = "home\0";
        assert_eq!(path_check(path, &mut is_goal), 4);
        assert!(
            is_goal,
            "Path with null character should set is_goal to true"
        );

        // // Test too long filename
        // let long_path = "a".repeat(EXT4_DIRECTORY_FILENAME_LEN + 10);
        // assert_eq!(ext4_path_check(&long_path, &mut is_goal), EXT4_DIRECTORY_FILENAME_LEN);
        // assert!(!is_goal, "Long filename should not set is_goal to true and should be truncated");
    }
}
