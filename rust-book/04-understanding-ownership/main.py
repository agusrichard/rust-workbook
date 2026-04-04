from typing import List

class Document:     
    def __init__(self, words: List[str]):
        """Create a new document"""
        self.words = words

    def add_word(self, word: str):
        """Add a word to the document"""
        self.words.append(word)
        
    def get_words(self) -> List[str]:  
        """Get a list of all the words in the document"""
        return self.words


words = ["Hello"]
d = Document(words)

d2 = Document(d.get_words())
d2.add_word("world")

print(words)
print(d.get_words())
print(d2.get_words())