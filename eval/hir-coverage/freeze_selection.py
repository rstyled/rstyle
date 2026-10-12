import urllib.request,hashlib,tarfile,io,pathlib,json
from tree_sitter import Language,Parser
import tree_sitter_rust
root=pathlib.Path('target/hir-coverage-frozen')
parser=Parser(Language(tree_sitter_rust.language()))
packages=[('lz4_flex','0.14.0','ecbdfe44b1bd960b68170b417450a628c43f7cf56bb3c5317e61cb230ee7f226'),('regex-syntax','0.8.11','d6f6ff9a378485b298a5286656da665ba74413d36db0979633275d2e708145d4'),('time-core','0.1.9','9e1c906769ad99c88eaa54e728060edef082f8e358ff32030cb7c7d315e81109')]
selections=[]
for name,version,digest in packages:
 data=urllib.request.urlopen(f'https://static.crates.io/crates/{name}/{name}-{version}.crate').read()
 assert hashlib.sha256(data).hexdigest()==digest
 tarfile.open(fileobj=io.BytesIO(data),mode='r:gz').extractall(root,filter='data')
 package=root/f'{name}-{version}'
 inventory=[]
 for path in sorted((package/'src').rglob('*.rs')):
  source=path.read_bytes()
  def walk(node):
   if node.type=='function_item':
    exact=source[node.start_byte:node.end_byte]
    inventory.append(dict(package=name,version=version,path=str(path.relative_to(package)),start=node.start_byte,end=node.end_byte,name=source[node.child_by_field_name('name').start_byte:node.child_by_field_name('name').end_byte].decode(),source=exact.decode(),sha256=hashlib.sha256(exact).hexdigest()))
   for child in node.children: walk(child)
  walk(parser.parse(source).root_node)
 inventory.sort(key=lambda f:(f['path'],f['start']))
 selections.extend(inventory[:24])
 print(name,len(inventory),'definitions;',len(inventory[:24]),'selected')
pathlib.Path('eval/hir-coverage/selection.json').write_text(json.dumps(selections,indent=2)+'\n')
