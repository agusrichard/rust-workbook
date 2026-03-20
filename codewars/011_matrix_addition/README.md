# Matrix Addition

Link to challenge: https://www.codewars.com/kata/526233aefd4764272800036f/train/rust

<div class="description-content p-4">
                <div class="markdown prose max-w-none mb-8" id="description"><p>Write a function that accepts two square matrices (<code>N x N</code> two dimensional arrays), and return the sum of the two. Both matrices being passed into the function will be of size <code>N x N</code> (square), containing only integers.</p>
<p>How to sum two matrices:</p>
<p>Take each cell <code>[n][m]</code> from the first matrix, and add it with the same <code>[n][m]</code> cell from the second matrix. This will be cell <code>[n][m]</code> of the solution matrix. (Except for C where solution matrix will be a 1d pseudo-multidimensional array).</p>
<p>Visualization: </p>
<pre><code>|1 2 3|     |2 2 1|     |1+2 2+2 3+1|     |3 4 4|
|3 2 1|  +  |3 2 3|  =  |3+3 2+2 1+3|  =  |6 4 4|
|1 1 1|     |1 1 3|     |1+1 1+1 1+3|     |2 2 4|
</code></pre>
<h2 id="example">Example</h2>
<pre><code class="language-javascript"><span class="cm-variable">matrixAddition</span>(
  [ [<span class="cm-number">1</span>, <span class="cm-number">2</span>, <span class="cm-number">3</span>],
    [<span class="cm-number">3</span>, <span class="cm-number">2</span>, <span class="cm-number">1</span>],
    [<span class="cm-number">1</span>, <span class="cm-number">1</span>, <span class="cm-number">1</span>] ],
<span class="cm-comment">//      +</span>
  [ [<span class="cm-number">2</span>, <span class="cm-number">2</span>, <span class="cm-number">1</span>],
    [<span class="cm-number">3</span>, <span class="cm-number">2</span>, <span class="cm-number">3</span>],
    [<span class="cm-number">1</span>, <span class="cm-number">1</span>, <span class="cm-number">3</span>] ] )

<span class="cm-comment">// returns:</span>
[ [<span class="cm-number">3</span>, <span class="cm-number">4</span>, <span class="cm-number">4</span>],
[<span class="cm-number">6</span>, <span class="cm-number">4</span>, <span class="cm-number">4</span>],
[<span class="cm-number">2</span>, <span class="cm-number">2</span>, <span class="cm-number">4</span>] ]
</code></pre>
</div>
                  <hr>
                  <div class="mt-4"><span><i class="icon-moon-tag "></i></span><div class="keyword-tag">Matrix</div><div class="keyword-tag">Arrays</div><div class="keyword-tag">Algorithms</div></div>
              </div>