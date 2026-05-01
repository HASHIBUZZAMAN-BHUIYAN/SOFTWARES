import tkinter as tk
from googletrans import Translator, LANGUAGES

import threading


def translate_text(event):
    text_to_translate = entry_english.get()
    if event.keysym == "BackSpace":
        translated_label.config(text="")
    else:
        if text_to_translate:
            translation_thread = threading.Thread(target=perform_translation, args=(text_to_translate,))
            translation_thread.start()


def perform_translation(text):
    translator = Translator()
    try:
        translated_text = translator.translate(text, dest='bn')
        detected_lang = translator.detect(text).lang
        detected_language = LANGUAGES.get(detected_lang)
        if detected_language:
            translated_label.config(text=f"Detected Language: {detected_language}\nTranslation: {translated_text.text}")
        else:
            translated_label.config(text=f"Translation: {translated_text.text}")
    except Exception as e:
        print("Error:", e)
        translated_label.config(text="Translation failed. Please try again.")


# Create the main window
window = tk.Tk()
window.title("Language Translator to Bangla")

# English input field and its title
english_frame = tk.Frame(window)
english_frame.pack(padx=10, pady=10)
english_title = tk.Label(english_frame, text="Enter text (any language)")
english_title.pack()
entry_english = tk.Entry(english_frame, width=40)
entry_english.pack()
entry_english.bind("<KeyRelease>", translate_text)  # Bind the translation function to KeyRelease event

# Bangla translation display field and its title
bangla_frame = tk.Frame(window)
bangla_frame.pack(padx=10, pady=10)
bangla_title = tk.Label(bangla_frame, text="Bangla Translation")
bangla_title.pack()
translated_label = tk.Label(bangla_frame, text="", wraplength=300)
translated_label.pack()

# Run the app
window.mainloop()
