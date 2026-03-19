# Valid Parentheses

Link to challenge: https://www.codewars.com/kata/6411b91a5e71b915d237332d/train/rust

<div class="markdown prose max-w-none mb-8" id="description"><p>Write a function that takes a string of parentheses, and determines if the order of the parentheses is valid. The function should return <code>true</code> if the string is valid, and <code>false</code> if it's invalid.</p>
<h2 id="examples">Examples</h2>
<pre><code>"()"              =&gt;  true
")(()))"          =&gt;  false
"("               =&gt;  false
"(())((()())())"  =&gt;  true
</code></pre>
<h2 id="constraints">Constraints</h2>
<p><code>0 &lt;= length of input &lt;= 100</code></p>
<ul>
<li>All inputs will be strings, consisting only of characters <code>(</code> and <code>)</code>.</li>
<li>Empty strings are considered balanced (and therefore valid), and will be tested.</li>
<li>For languages with mutable strings, the inputs should not be mutated.</li>
</ul>
<br>

<p><em><strong>Protip:</strong> If you are trying to figure out why a string of parentheses is invalid, paste the parentheses into the code editor, and let the code highlighting show you!</em></p>
</div>