# Disease Spread

Link to challenge: https://www.codewars.com/kata/566543703c72200f0b0000c9/train/rust

<div class="description-content p-4">
                <div class="markdown prose max-w-none mb-8" id="description"><p>In 1978 the British Medical Journal reported on an outbreak of influenza at a British boarding school. There were <code>1000</code> students. The outbreak began with one infected student.</p>
<p>We want to study the spread of the disease through the population of this school. The total population may be divided into three: 
the infected <code>(i)</code>, those who have recovered <code>(r)</code>, and
those who are still susceptible <code>(s)</code> to get the disease.</p>
<p>We will study the disease on a period of <code>tm</code> days. One model of propagation uses 3 differential equations:</p>
<pre><code>(1) s'(t) = -b * s(t) * i(t)
(2) i'(t) =  b * s(t) * i(t) - a * i(t)
(3) r'(t) =  a * i(t)
</code></pre>
<p>where <code>s(t), i(t), r(t)</code> are the susceptible, infected, recovered at time <code>t</code> and 
<code>s'(t), i'(t), r'(t)</code> the corresponding derivatives. 
<code>b</code> and <code>a</code> are constants:
<code>b</code> is representing a number of contacts which can spread the disease and
<code>a</code> is a fraction of the infected that will recover. </p>
<p>We can transform equations <code>(1), (2), (3)</code> in finite differences
    (<a href="https://en.wikipedia.org/wiki/Finite_difference_method#Example:_ordinary_differential_equation" data-turbolinks="false" target="_blank">https://en.wikipedia.org/wiki/Finite_difference_method#Example:_ordinary_differential_equation</a>)
    (<a href="http://www.codewars.com/kata/56347fcfd086de8f11000014" data-turbolinks="false" target="_blank">http://www.codewars.com/kata/56347fcfd086de8f11000014</a>)</p>
<pre><code>(I)    S[k+1] = S[k] - dt * b * S[k] * I[k]
(II)   I[k+1] = I[k] + dt * (b * S[k] * I[k] - a * I[k])
(III)  R[k+1] = R[k] + dt * I[k] *a
</code></pre>
<p>The interval <code>[0, tm]</code> will be divided in <code>n</code> small intervals of length <code>dt = tm/n</code>.
Initial conditions here could be : <code>S0 = 999, I0 = 1, R0 = 0</code>
Whatever S0 and I0, R0 (number of recovered at time 0) is always 0.</p>
<p>The function <code>epidemic</code> will return the maximum number of infected
as an <em>integer</em> (truncate to integer the result of max(I)).</p>
<h4 id="example">Example:</h4>
<pre><code>tm = 14 ;n = 336 ;s0 = 996 ;i0 = 2 ;b = 0.00206 ;a = 0.41
epidemic(tm, n, s0, i0, b, a) --&gt; 483
</code></pre>
<h4 id="notes">Notes:</h4>
<ul>
<li>Keeping track of the values of susceptible, infected and recovered you can plot the solutions of the 3 differential equations. See an example below on the plot.</li>
</ul>
<p><img alt="alternative text" src="http://i.imgur.com/xB6VSqzm.png"></p>
</div>
                  <hr>
              </div>