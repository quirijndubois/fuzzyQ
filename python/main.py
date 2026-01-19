import sys
import os
import readchar
import time

# linux
terminal_size = os.get_terminal_size()
terminal_width = terminal_size.columns
terminal_height = terminal_size.lines

def clear_line():
    sys.stdout.write('\r' + ' ' * (terminal_width - 1) + '\r')
    sys.stdout.flush()

def move_cursor_up(lines=1):
    sys.stdout.write(f'\033[{lines}A')
    sys.stdout.flush()

def move_cursor_down(lines=1):
    sys.stdout.write(f'\033[{lines}B')
    sys.stdout.flush()

def move_cursor_left(chars = 1):
    sys.stdout.write(f'\033[{chars}D')
    sys.stdout.flush()

def move_cursor_right(chars = 1):
    sys.stdout.write(f'\033[{chars}C')
    sys.stdout.flush()

def move_cursor_to_start():
    sys.stdout.write('\r')
    sys.stdout.flush()

def move_cursor_to_end():
    move_cursor_to_start()
    sys.stdout.write(f'\033[{terminal_width}C')
    sys.stdout.flush()

def write_text(text, color = '\033[0m'):
    sys.stdout.write(color)
    sys.stdout.write(text)
    sys.stdout.flush()

def write_entry(text, match_indeces):
    last_index = 0
    for start, end in match_indeces:
        write_text(text[last_index:start])
        write_text(text[start:end], '\033[1;32m')
        last_index = end
    write_text(text[last_index:])

def get_suggestions(search_term, options):
    suggestions = []
    for option in options:
        match_length = 0
        for letter in range(min(len(search_term), len(option))):
            if search_term[letter] == option[letter]:
                match_length += 1
            else:
                break
        match_indeces = [(0, match_length)] 
        match = match_length
        entry = {
            'text': option,
            'match_indeces': match_indeces,
            'match' : match
        }
        suggestions.append(entry)

    suggestions = sorted(suggestions, key=lambda x: x['match'], reverse=True)
    return suggestions

typed = ""
last_suggestion_amount = 0
stop = False
with open("words.txt", "r") as f:
    sample_options = [line.strip() for line in f.readlines()]

while not stop:
    char = readchar.readchar()
    if char == '\n':
        stop = True
    elif char == '\x7f':  # Backspace
        typed = typed[:-1]
    elif char == '\x03':  # Ctrl-C
        stop = True
    elif char in ('\x1b', '\x1b[A', '\x1b[B', '\x1b[C', '\x1b[D'):
        continue
    else:
        typed += char

    begin_time = time.time()
    suggestions = get_suggestions(typed, sample_options)[:20]
    delta_time = (time.time() - begin_time) * 1000
    delta_time_str = f"{delta_time:.2f}ms"

    for _ in range(last_suggestion_amount):
        move_cursor_down()
        clear_line()
    for _ in range(last_suggestion_amount):
        move_cursor_up()

    for suggestion in suggestions:
        move_cursor_down()
        clear_line()
        write_entry(suggestion['text'], suggestion['match_indeces'])
    for _ in suggestions:
        move_cursor_up()
    last_suggestion_amount = len(suggestions)
 
    clear_line()
     # drawing delta time
    move_cursor_to_end()
    move_cursor_left(len(delta_time_str)+1)
    write_text(delta_time_str, '\033[90m')

    move_cursor_to_start()
    write_text("Type here: ")
    write_text(typed)

print()













