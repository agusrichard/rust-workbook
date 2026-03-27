# Luck check

Link to challenge: https://www.codewars.com/kata/5314b3c6bb244a48ab00076c

<div class="description-content p-4">
                <div class="markdown prose max-w-none mb-8" id="description"><p>In some countries of former Soviet Union there was a belief about lucky tickets. A transport ticket of any sort was believed to posess luck if sum of digits on the left half of its number was equal to the sum of digits on the right half. Here are examples of such numbers:</p>
<pre><code>003111    #             3 = 1 + 1 + 1
813372    #     8 + 1 + 3 = 3 + 7 + 2
17935     #         1 + 7 = 3 + 5  // if the length is odd, you should ignore the middle number when adding the halves.
56328116  # 5 + 6 + 3 + 2 = 8 + 1 + 1 + 6
</code></pre>
<p>Such tickets were either eaten after being used or collected for bragging rights.</p>
<p>Your task is to write a funtion <code>luck_check(str)</code>, which returns <code>true/True</code> if argument is string decimal representation of a lucky ticket number, or <code>false/False</code> for all other numbers. It should throw errors for empty strings or strings which don't represent a decimal number.</p>
</div>
                  <hr>
                  <div class="mt-4"><span><i class="icon-moon-tag "></i></span><div class="keyword-tag">Strings</div><div class="keyword-tag">Mathematics</div><div class="keyword-tag">Puzzles</div></div>
              </div>
