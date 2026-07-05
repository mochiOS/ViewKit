#define _POSIX_C_SOURCE 200809L

#include <ctype.h>
#include <errno.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "viewkit_abi.h"

#define WINDOW_TITLE "ViewKitExample: mochiOS Builder"

#define ROOT_NODE_ID 1
#define FIRST_DYNAMIC_NODE_ID 100

#define STATE_CONFIG_PATH 1
#define FIRST_ENTRY_STATE_ID 1000
#define STATE_EDITOR_SCROLL 900000

typedef enum ConfigLineKind {
    CONFIG_LINE_RAW,
    CONFIG_LINE_ENTRY,
} ConfigLineKind;

typedef struct ConfigEntry {
    char *key;
    char *label;
    char *value;

    bool is_boolean;
    bool boolean_value;

    uint64_t state_id;
} ConfigEntry;

typedef struct ConfigLine {
    ConfigLineKind kind;

    union {
        char *raw;
        ConfigEntry entry;
    };
} ConfigLine;

typedef struct ConfigDocument {
    ConfigLine *lines;

    size_t count;
    size_t capacity;
    size_t entry_count;
} ConfigDocument;

static uint64_t next_node_id =
    FIRST_DYNAMIC_NODE_ID;

static void die(
    const char *message
)
{
    fprintf(
        stderr,
        "error: %s\n",
        message
    );

    exit(EXIT_FAILURE);
}

static void die_errno(
    const char *operation
)
{
    fprintf(
        stderr,
        "error: %s: %s\n",
        operation,
        strerror(errno)
    );

    exit(EXIT_FAILURE);
}

static void *xmalloc(
    size_t size
)
{
    void *pointer =
        malloc(size);

    if (pointer == NULL) {
        die("out of memory");
    }

    return pointer;
}

static void *xrealloc(
    void *pointer,
    size_t size
)
{
    void *result =
        realloc(pointer, size);

    if (result == NULL) {
        die("out of memory");
    }

    return result;
}

static char *xstrdup(
    const char *value
)
{
    char *copy =
        strdup(value);

    if (copy == NULL) {
        die("out of memory");
    }

    return copy;
}

static VkString vk_string(
    const char *value
)
{
    if (value == NULL ||
        value[0] == '\0') {
        return (VkString) {
            .pointer = NULL,
            .length = 0,
        };
    }

    return (VkString) {
        .pointer =
            (const uint8_t *)value,

        .length =
            strlen(value),
    };
}

static VkLength vk_length_auto(
    void
)
{
    return (VkLength) {
        .kind = VK_LENGTH_AUTO,
        .value = 0.0f,
    };
}

static VkLength vk_length_fixed(
    float value
)
{
    return (VkLength) {
        .kind = VK_LENGTH_FIXED,
        .value = value,
    };
}

static void fail_status(
    const char *operation,
    int32_t status
)
{
    VkString name =
        vk_status_name(status);

    fprintf(
        stderr,
        "%s failed: %.*s (%d)\n",
        operation,
        (int)name.length,
        (const char *)name.pointer,
        status
    );

    exit(EXIT_FAILURE);
}

#define VK_CHECK(expression)                 \
    do {                                     \
        int32_t status__ = (expression);     \
                                             \
        if (status__ != 0) {                 \
            fail_status(                     \
                #expression,                 \
                status__                     \
            );                               \
        }                                    \
    } while (0)

static void reset_node_ids(
    void
)
{
    next_node_id =
        FIRST_DYNAMIC_NODE_ID;
}

static uint64_t allocate_node_id(
    void
)
{
    return next_node_id++;
}

static bool word_is_all_uppercase(
    const char *word
)
{
    bool found_letter = false;

    for (const unsigned char *cursor =
             (const unsigned char *)word;
         *cursor != '\0';
         cursor++) {
        if (!isalpha(*cursor)) {
            continue;
        }

        found_letter = true;

        if (!isupper(*cursor)) {
            return false;
        }
    }

    return found_letter;
}

static bool word_contains_digit(
    const char *word
)
{
    for (const unsigned char *cursor =
             (const unsigned char *)word;
         *cursor != '\0';
         cursor++) {
        if (isdigit(*cursor)) {
            return true;
        }
    }

    return false;
}

static bool should_preserve_uppercase(
    const char *word
)
{
    size_t length =
        strlen(word);

    return
        word_is_all_uppercase(word) &&
        (
            length <= 4 ||
            word_contains_digit(word)
        );
}

static char *humanize_key(
    const char *key
)
{
    const char *source = key;

    if (strncmp(
        source,
        "CONFIG_",
        7
    ) == 0) {
        source += 7;
    }

    char *copy =
        xstrdup(source);

    size_t capacity =
        strlen(source) + 1;

    char *output =
        xmalloc(capacity);

    size_t output_length = 0;

    char *save_pointer = NULL;

    char *word =
        strtok_r(
            copy,
            "_",
            &save_pointer
        );

    while (word != NULL) {
        if (output_length != 0) {
            output[output_length++] = ' ';
        }

        size_t word_length =
            strlen(word);

        if (should_preserve_uppercase(
            word
        )) {
            memcpy(
                output + output_length,
                word,
                word_length
            );

            output_length +=
                word_length;
        } else {
            for (size_t index = 0;
                 index < word_length;
                 index++) {
                unsigned char character =
                    (unsigned char)word[index];

                if (index == 0) {
                    output[output_length++] =
                        (char)toupper(character);
                } else {
                    output[output_length++] =
                        (char)tolower(character);
                }
            }
        }

        word =
            strtok_r(
                NULL,
                "_",
                &save_pointer
            );
    }

    output[output_length] = '\0';

    free(copy);

    return output;
}

static void remove_line_ending(
    char *line
)
{
    size_t length =
        strlen(line);

    while (length != 0) {
        char character =
            line[length - 1];

        if (character != '\n' &&
            character != '\r') {
            break;
        }

        line[--length] = '\0';
    }
}

static char *trimmed_copy(
    const char *value
)
{
    const unsigned char *begin =
        (const unsigned char *)value;

    while (*begin != '\0' &&
           isspace(*begin)) {
        begin++;
    }

    const unsigned char *end =
        begin + strlen(
            (const char *)begin
        );

    while (end > begin &&
           isspace(end[-1])) {
        end--;
    }

    size_t length =
        (size_t)(end - begin);

    char *result =
        xmalloc(length + 1);

    memcpy(
        result,
        begin,
        length
    );

    result[length] = '\0';

    return result;
}

static void document_append(
    ConfigDocument *document,
    ConfigLine line
)
{
    if (document->count ==
        document->capacity) {
        size_t new_capacity =
            document->capacity == 0
                ? 32
                : document->capacity * 2;

        document->lines =
            xrealloc(
                document->lines,
                new_capacity *
                    sizeof(ConfigLine)
            );

        document->capacity =
            new_capacity;
    }

    document->lines[
        document->count++
    ] = line;
}

static bool parse_config_entry(
    const char *line,
    size_t entry_index,
    ConfigEntry *output
)
{
    if (strncmp(
        line,
        "CONFIG_",
        7
    ) != 0) {
        return false;
    }

    const char *separator =
        strchr(line, '=');

    if (separator == NULL ||
        separator == line) {
        return false;
    }

    size_t key_length =
        (size_t)(separator - line);

    char *key =
        xmalloc(key_length + 1);

    memcpy(
        key,
        line,
        key_length
    );

    key[key_length] = '\0';

    char *value =
        xstrdup(separator + 1);

    bool is_boolean =
        strcmp(value, "y") == 0 ||
        strcmp(value, "n") == 0;

    *output = (ConfigEntry) {
        .key = key,

        .label =
            humanize_key(key),

        .value = value,

        .is_boolean =
            is_boolean,

        .boolean_value =
            strcmp(value, "y") == 0,

        .state_id =
            FIRST_ENTRY_STATE_ID +
            entry_index,
    };

    return true;
}

static ConfigDocument read_config(
    const char *path
)
{
    FILE *file =
        fopen(path, "r");

    if (file == NULL) {
        die_errno(path);
    }

    ConfigDocument document = {0};

    char *line = NULL;
    size_t line_capacity = 0;

    while (getline(
        &line,
        &line_capacity,
        file
    ) >= 0) {
        remove_line_ending(line);

        ConfigEntry entry;

        if (parse_config_entry(
            line,
            document.entry_count,
            &entry
        )) {
            document_append(
                &document,
                (ConfigLine) {
                    .kind =
                        CONFIG_LINE_ENTRY,

                    .entry =
                        entry,
                }
            );

            document.entry_count++;
        } else {
            document_append(
                &document,
                (ConfigLine) {
                    .kind =
                        CONFIG_LINE_RAW,

                    .raw =
                        xstrdup(line),
                }
            );
        }
    }

    if (ferror(file)) {
        free(line);
        fclose(file);

        die_errno(
            "reading configuration"
        );
    }

    free(line);

    if (fclose(file) != 0) {
        die_errno(
            "closing configuration"
        );
    }

    return document;
}

static void free_document(
    ConfigDocument *document
)
{
    for (size_t index = 0;
         index < document->count;
         index++) {
        ConfigLine *line =
            &document->lines[index];

        if (line->kind ==
            CONFIG_LINE_RAW) {
            free(line->raw);
            continue;
        }

        free(line->entry.key);
        free(line->entry.label);
        free(line->entry.value);
    }

    free(document->lines);

    *document =
        (ConfigDocument){0};
}

static VkRectangleStyle card_style(
    void
)
{
    return (VkRectangleStyle) {
        .color_kind =
            VK_RECTANGLE_COLOR_SURFACE,

        .custom_color = {
            .red = 0,
            .green = 0,
            .blue = 0,
            .alpha = 0,
        },

        .radius_kind =
            VK_CORNER_RADIUS_CARD,

        .radius = 0.0f,

        .border_kind =
            VK_BORDER_STANDARD,

        .border_color = {
            .red = 0,
            .green = 0,
            .blue = 0,
            .alpha = 0,
        },

        .border_width = 0.0f,
    };
}

static void push_text(
    VkRuntime *runtime,
    const char *content,
    float font_size,
    float line_height,
    uint16_t weight
)
{
    VK_CHECK(
        vk_push_text(
            runtime,
            allocate_node_id(),
            vk_string(content),
            font_size,
            line_height,
            weight,
            VK_TEXT_ALIGNMENT_START,
            VK_TEXT_COLOR_BLACK
        )
    );
}

static void begin_page(
    VkRuntime *runtime,
    bool scrollable
)
{
    VK_CHECK(
        vk_tree_begin(
            runtime,
            ROOT_NODE_ID
        )
    );

    if (scrollable) {
        VK_CHECK(
            vk_begin_scroll(
                runtime,
                allocate_node_id(),
                STATE_EDITOR_SCROLL,
                VK_SCROLL_AXIS_VERTICAL,
                VK_SCROLLBAR_AUTOMATIC
            )
        );
    }

    VK_CHECK(
        vk_begin_padding(
            runtime,
            allocate_node_id(),
            32.0f,
            40.0f,
            40.0f,
            40.0f
        )
    );

    VK_CHECK(
        vk_begin_vstack(
            runtime,
            allocate_node_id(),
            VK_STACK_GAP_LARGE,
            VK_ALIGNMENT_STRETCH,
            VK_DISTRIBUTION_START
        )
    );
}

static void end_page(
    VkRuntime *runtime,
    bool scrollable
)
{
    VK_CHECK(vk_end_node(runtime));
    VK_CHECK(vk_end_node(runtime));

    if (scrollable) {
        VK_CHECK(vk_end_node(runtime));
    }

    VK_CHECK(
        vk_tree_commit(runtime)
    );
}

static void begin_card(
    VkRuntime *runtime
)
{
    VK_CHECK(
        vk_begin_card(
            runtime,
            allocate_node_id(),
            card_style()
        )
    );

    VK_CHECK(
        vk_begin_padding(
            runtime,
            allocate_node_id(),
            24.0f,
            24.0f,
            24.0f,
            24.0f
        )
    );

    VK_CHECK(
        vk_begin_vstack(
            runtime,
            allocate_node_id(),
            VK_STACK_GAP_MEDIUM,
            VK_ALIGNMENT_STRETCH,
            VK_DISTRIBUTION_START
        )
    );
}

static void end_card(
    VkRuntime *runtime
)
{
    VK_CHECK(vk_end_node(runtime));
    VK_CHECK(vk_end_node(runtime));
    VK_CHECK(vk_end_node(runtime));
}

static void push_path_field(
    VkRuntime *runtime,
    const char *initial_path
)
{
    VK_CHECK(
        vk_begin_frame(
            runtime,
            allocate_node_id(),
            vk_length_fixed(620.0f),
            vk_length_auto()
        )
    );

    VK_CHECK(
        vk_push_text_field(
            runtime,
            allocate_node_id(),
            STATE_CONFIG_PATH,
            vk_string(initial_path),
            vk_string(
                "/path/to/.config"
            ),
            VK_TEXT_FIELD_SIZE_MEDIUM,
            10.0f,
            1,
            0
        )
    );

    VK_CHECK(vk_end_node(runtime));
}

static char *read_string_state(
    VkRuntime *runtime,
    uint64_t state_id
)
{
    size_t length = 0;

    VK_CHECK(
        vk_state_string_length(
            runtime,
            state_id,
            &length
        )
    );

    char *value =
        xmalloc(length + 1);

    size_t written = 0;

    VK_CHECK(
        vk_state_copy_string(
            runtime,
            state_id,
            (uint8_t *)value,
            length,
            &written
        )
    );

    value[written] = '\0';

    return value;
}

static VkRuntime *create_runtime(
    void
)
{
    VkRuntime *runtime =
        vk_runtime_create(1);

    if (runtime == NULL) {
        die(
            "could not create ViewKit runtime"
        );
    }

    return runtime;
}

static void destroy_runtime(
    VkRuntime *runtime
)
{
    VK_CHECK(
        vk_runtime_destroy(runtime)
    );
}

static char *choose_config_path(
    const char *initial_path
)
{
    reset_node_ids();

    VkRuntime *runtime =
        create_runtime();

    begin_page(
        runtime,
        false
    );

    push_text(
        runtime,
        "mochiOS Builder",
        28.0f,
        36.0f,
        700
    );

    push_text(
        runtime,
        "Enter the path to a mochiOS .config file.",
        15.0f,
        23.0f,
        400
    );

    begin_card(runtime);

    push_text(
        runtime,
        "ConfigFile",
        20.0f,
        28.0f,
        650
    );

    push_path_field(
        runtime,
        initial_path
    );

    push_text(
        runtime,
        "Close this window to open the selected configuration. Leave the field empty to cancel.",
        13.0f,
        20.0f,
        400
    );

    end_card(runtime);

    end_page(
        runtime,
        false
    );

    int32_t status =
        vk_runtime_run_window(
            runtime,
            vk_string(WINDOW_TITLE),
            720.0f,
            330.0f,
            0
        );

    if (status != 0) {
        destroy_runtime(runtime);

        fail_status(
            "vk_runtime_run_window",
            status
        );
    }

    char *raw_path =
        read_string_state(
            runtime,
            STATE_CONFIG_PATH
        );

    char *path =
        trimmed_copy(raw_path);

    free(raw_path);

    destroy_runtime(runtime);

    return path;
}

static void push_config_row(
    VkRuntime *runtime,
    const ConfigEntry *entry
)
{
    VK_CHECK(
        vk_begin_hstack(
            runtime,
            allocate_node_id(),
            VK_STACK_GAP_MEDIUM,
            VK_ALIGNMENT_CENTER,
            VK_DISTRIBUTION_START
        )
    );

    push_text(
        runtime,
        entry->label,
        15.0f,
        22.0f,
        500
    );

    VK_CHECK(
        vk_push_spacer(
            runtime,
            allocate_node_id()
        )
    );

    if (entry->is_boolean) {
        VK_CHECK(
            vk_push_switch(
                runtime,
                allocate_node_id(),
                entry->state_id,
                entry->boolean_value
                    ? 1
                    : 0,
                vk_string(""),
                1
            )
        );
    } else {
        VK_CHECK(
            vk_begin_frame(
                runtime,
                allocate_node_id(),
                vk_length_fixed(320.0f),
                vk_length_auto()
            )
        );

        VK_CHECK(
            vk_push_text_field(
                runtime,
                allocate_node_id(),
                entry->state_id,
                vk_string(entry->value),
                vk_string("Value"),
                VK_TEXT_FIELD_SIZE_MEDIUM,
                10.0f,
                1,
                0
            )
        );

        VK_CHECK(vk_end_node(runtime));
    }

    VK_CHECK(vk_end_node(runtime));
}

static void build_editor_ui(
    VkRuntime *runtime,
    const ConfigDocument *document,
    const char *config_path
)
{
    reset_node_ids();

    begin_page(
        runtime,
        true
    );

    push_text(
        runtime,
        "mochiOS Builder",
        28.0f,
        36.0f,
        700
    );

    push_text(
        runtime,
        "Edit the generated mochiOS build configuration.",
        15.0f,
        23.0f,
        400
    );

    begin_card(runtime);

    push_text(
        runtime,
        "File",
        20.0f,
        28.0f,
        650
    );

    push_path_field(
        runtime,
        config_path
    );

    push_text(
        runtime,
        "The edited configuration will be written to this path when the window closes.",
        13.0f,
        20.0f,
        400
    );

    end_card(runtime);

    begin_card(runtime);

    push_text(
        runtime,
        "Build Configuration",
        20.0f,
        28.0f,
        650
    );

    if (document->entry_count == 0) {
        push_text(
            runtime,
            "No CONFIG_ entries were found.",
            14.0f,
            21.0f,
            400
        );
    }

    size_t rendered_entries = 0;

    for (size_t index = 0;
         index < document->count;
         index++) {
        const ConfigLine *line =
            &document->lines[index];

        if (line->kind !=
            CONFIG_LINE_ENTRY) {
            continue;
        }

        if (rendered_entries != 0) {
            VK_CHECK(
                vk_push_divider(
                    runtime,
                    allocate_node_id()
                )
            );
        }

        push_config_row(
            runtime,
            &line->entry
        );

        rendered_entries++;
    }

    end_card(runtime);

    push_text(
        runtime,
        "Boolean y/n values are shown as switches. Other values are edited as text.",
        13.0f,
        20.0f,
        400
    );

    end_page(
        runtime,
        true
    );
}

static bool synchronize_document(
    VkRuntime *runtime,
    ConfigDocument *document
)
{
    bool changed = false;

    for (size_t index = 0;
         index < document->count;
         index++) {
        ConfigLine *line =
            &document->lines[index];

        if (line->kind !=
            CONFIG_LINE_ENTRY) {
            continue;
        }

        ConfigEntry *entry =
            &line->entry;

        if (entry->is_boolean) {
            uint8_t state_value = 0;

            VK_CHECK(
                vk_state_get_bool(
                    runtime,
                    entry->state_id,
                    &state_value
                )
            );

            bool value =
                state_value != 0;

            if (value !=
                entry->boolean_value) {
                entry->boolean_value =
                    value;

                free(entry->value);

                entry->value =
                    xstrdup(
                        value
                            ? "y"
                            : "n"
                    );

                changed = true;
            }

            continue;
        }

        char *value =
            read_string_state(
                runtime,
                entry->state_id
            );

        if (strcmp(
            value,
            entry->value
        ) == 0) {
            free(value);
            continue;
        }

        free(entry->value);

        entry->value = value;

        changed = true;
    }

    return changed;
}

static void write_config(
    const char *path,
    const ConfigDocument *document
)
{
    size_t temporary_capacity =
        strlen(path) + 64;

    char *temporary_path =
        xmalloc(
            temporary_capacity
        );

    snprintf(
        temporary_path,
        temporary_capacity,
        "%s.tmp.%ld",
        path,
        (long)getpid()
    );

    FILE *file =
        fopen(
            temporary_path,
            "w"
        );

    if (file == NULL) {
        free(temporary_path);

        die_errno(
            "opening temporary configuration"
        );
    }

    for (size_t index = 0;
         index < document->count;
         index++) {
        const ConfigLine *line =
            &document->lines[index];

        int result;

        if (line->kind ==
            CONFIG_LINE_RAW) {
            result =
                fprintf(
                    file,
                    "%s\n",
                    line->raw
                );
        } else {
            result =
                fprintf(
                    file,
                    "%s=%s\n",
                    line->entry.key,
                    line->entry.value
                );
        }

        if (result < 0) {
            fclose(file);
            remove(temporary_path);
            free(temporary_path);

            die_errno(
                "writing configuration"
            );
        }
    }

    if (fflush(file) != 0) {
        fclose(file);
        remove(temporary_path);
        free(temporary_path);

        die_errno(
            "flushing configuration"
        );
    }

    if (fclose(file) != 0) {
        remove(temporary_path);
        free(temporary_path);

        die_errno(
            "closing configuration"
        );
    }

    if (rename(
        temporary_path,
        path
    ) != 0) {
        remove(temporary_path);
        free(temporary_path);

        die_errno(
            "replacing configuration"
        );
    }

    free(temporary_path);
}

int main(
    int argc,
    char **argv
)
{
    if (argc > 2) {
        fprintf(
            stderr,
            "usage: %s [config-path]\n",
            argv[0]
        );

        return EXIT_FAILURE;
    }

    if (argc == 1) {
        char *selected_path =
            choose_config_path("");

        if (selected_path[0] == '\0') {
            free(selected_path);

            return EXIT_SUCCESS;
        }

        char *const new_arguments[] = {
            argv[0],
            selected_path,
            NULL,
        };

        execvp(
            argv[0],
            new_arguments
        );

        fprintf(
            stderr,
            "error: could not restart %s: %s\n",
            argv[0],
            strerror(errno)
        );

        free(selected_path);

        return EXIT_FAILURE;
    }

    /*
     * argc == 2の場合はパス選択画面を出さず、
     * 設定編集ウィンドウを直接開く。
     */
    char *config_path =
        trimmed_copy(argv[1]);

    if (config_path[0] == '\0') {
        fprintf(
            stderr,
            "error: configuration path is empty\n"
        );

        free(config_path);

        return EXIT_FAILURE;
    }

    ConfigDocument document =
        read_config(config_path);

    printf(
        "loaded %zu configuration entries from %s\n",
        document.entry_count,
        config_path
    );

    VkRuntime *runtime =
        create_runtime();

    build_editor_ui(
        runtime,
        &document,
        config_path
    );

    int32_t status =
        vk_runtime_run_window(
            runtime,
            vk_string(WINDOW_TITLE),
            860.0f,
            720.0f,
            1
        );

    if (status != 0) {
        destroy_runtime(runtime);
        free_document(&document);
        free(config_path);

        fail_status(
            "vk_runtime_run_window",
            status
        );
    }

    /*
     * エディタ上部のパス欄から保存先を取得する。
     */
    char *raw_output_path =
        read_string_state(
            runtime,
            STATE_CONFIG_PATH
        );

    char *output_path =
        trimmed_copy(
            raw_output_path
        );

    free(raw_output_path);

    bool changed =
        synchronize_document(
            runtime,
            &document
        );

    destroy_runtime(runtime);

    if (output_path[0] == '\0') {
        fprintf(
            stderr,
            "configuration was not saved: "
            "output path is empty\n"
        );
    } else {
        bool output_path_changed =
            strcmp(
                output_path,
                config_path
            ) != 0;

        if (changed ||
            output_path_changed) {
            write_config(
                output_path,
                &document
            );

            printf(
                "saved configuration to %s\n",
                output_path
            );
        } else {
            printf(
                "configuration was not changed\n"
            );
        }
    }

    free(output_path);
    free(config_path);

    free_document(&document);

    return EXIT_SUCCESS;
}