import test from 'ava'

import { NodejsLspFiles } from '../index.js'

test('main class', (t) => {
  t.is(new NodejsLspFiles().getVariables("id", 11), null);
})
