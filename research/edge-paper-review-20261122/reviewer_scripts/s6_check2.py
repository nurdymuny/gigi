import sys, inspect
sys.path.insert(0,'/Users/bdavis/.claude/jobs/c8f0f39d/tmp/rtc/scripts')
sys.argv=[sys.argv[0]]
import ricci_su2_family_block_cert as FB
import sympy as sp
S=FB.symbolic()
# sources of nonzero C offdiag / Lp
for i in range(9):
  for k in range(9):
    if S['Cnz'][i][k] and i!=k: print('C',i,k,inspect.getsource(S['C'][i][k]).split('return')[1].strip())
for i in range(9):
  for k in range(9):
    if S['Lpnz'][i][k]: print('Lp',i,k,inspect.getsource(S['Lp'][i][k]).split('return')[1].strip())
    if S['Lmnz'][i][k]: print('Lm',i,k,inspect.getsource(S['Lm'][i][k]).split('return')[1].strip())
for a in range(9): print('Caa',a,S['ch'][a],inspect.getsource(S['C'][a][a]).split('return')[1].strip(), ' w:',inspect.getsource(S['w'][a]).split('return')[1].strip())
